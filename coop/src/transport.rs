use super::*;
use iroh::{Endpoint,endpoint::{Connection,RecvStream,SendStream,presets}};
use std::{path::PathBuf,sync::{Arc,atomic::{AtomicBool,Ordering},mpsc::{self,Receiver,SyncSender}},time::{Duration,Instant}};
use tokio::sync::{mpsc as channel,oneshot};

#[derive(Clone)]
pub struct Options{
    pub name:String,pub identity:String,pub force_relay:bool,
    /// Hashed on the worker, never read on a render frame. Missing files fail closed.
    pub identity_files:Vec<PathBuf>,
}
impl Options{pub fn new(name:impl Into<String>,identity:impl Into<String>)->Self{Self{name:name.into(),identity:identity.into(),force_relay:false,identity_files:Vec::new()}}}
pub struct Client{tx:channel::Sender<Command>,rx:Receiver<Event>,stop:Arc<AtomicBool>}
impl Client{
    pub fn host(options:Options,settings:Settings)->Self{Self::spawn(options,Some(settings),None)}
    pub fn join(options:Options,invite:Invite)->Self{Self::spawn(options,None,Some(invite))}
    fn spawn(options:Options,settings:Option<Settings>,invite:Option<Invite>)->Self{
        let (tx,commands)=channel::channel(64);let (events,rx)=mpsc::sync_channel(256);
        let stop=Arc::new(AtomicBool::new(false));let shutdown=stop.clone();let failed=events.clone();
        if std::thread::Builder::new().name("private-coop".into()).spawn(move||{
            let result=tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build();
            match result{
                Ok(runtime)=>{if let Err(error)=runtime.block_on(run(options,settings,invite,commands,&events,shutdown)){emit(&events,Event::Error(error));}},
                Err(_)=>emit(&events,Event::Error("Could not start networking".into())),
            }
        }).is_err(){emit(&failed,Event::Error("Could not start the connection worker".into()));}
        Self{tx,rx,stop}
    }
    pub fn send(&self,command:Command)->Result<(),String>{self.tx.try_send(command).map_err(|_|"Connection is busy or closed".into())}
    pub fn is_closed(&self)->bool{self.tx.is_closed()}
    pub fn poll(&self)->impl Iterator<Item=Event>+'_ {self.rx.try_iter()}
}
impl Drop for Client{fn drop(&mut self){self.stop.store(true,Ordering::Relaxed);let _=self.tx.try_send(Command::Leave);}}
fn emit(events:&SyncSender<Event>,event:Event){let _=events.try_send(event);}
fn error(e:impl std::fmt::Display)->String{format!("{e}")}
async fn read(recv:&mut RecvStream)->Result<Wire,String>{
    let mut length=[0;4];recv.read_exact(&mut length).await.map_err(error)?;
    let length=u32::from_le_bytes(length) as usize;
    if length==0||length>MAX_FRAME{return Err("Invalid network frame length".into());}
    let mut body=vec![0;length];recv.read_exact(&mut body).await.map_err(error)?;
    serde_json::from_slice(&body).map_err(|_|"Invalid network message".into())
}
async fn write(send:&mut SendStream,message:&Wire)->Result<(),String>{
    let body=serde_json::to_vec(message).map_err(error)?;
    if body.len()>MAX_FRAME{return Err("Network message too large".into());}
    tokio::time::timeout(Duration::from_secs(8),async{
        send.write_all(&(body.len() as u32).to_le_bytes()).await.map_err(error)?;
        send.write_all(&body).await.map_err(error)
    }).await.map_err(|_|"Connection timed out".to_owned())?
}
async fn run(mut options:Options,settings:Option<Settings>,invite:Option<Invite>,commands:channel::Receiver<Command>,events:&SyncSender<Event>,stop:Arc<AtomicBool>)->Result<(),String>{
    if !options.identity_files.is_empty(){
        use std::io::Read;
        let mut hash=blake3::Hasher::new();hash.update(options.identity.as_bytes());
        let mut buffer=[0;64*1024];
        for path in &options.identity_files{
            let mut file=std::fs::File::open(path).map_err(|_|"Game data could not be checked".to_owned())?;
            hash.update(&file.metadata().map_err(error)?.len().to_le_bytes());
            loop{let n=file.read(&mut buffer).map_err(error)?;if n==0{break;}hash.update(&buffer[..n]);}
        }
        options.identity=hash.finalize().to_hex().to_string();
    }
    let mut builder=Endpoint::builder(presets::N0).alpns(vec![ALPN.to_vec()]);
    if options.force_relay{builder=builder.clear_ip_transports();}
    let endpoint=builder.bind().await.map_err(|_|"Could not open the peer connection".to_owned())?;
    let work=async{
        tokio::time::timeout(Duration::from_secs(25),endpoint.online()).await.map_err(|_|"Relay service could not be reached. Check your internet connection.".to_owned())?;
        if let Some(settings)=settings{host(&endpoint,&options,settings,commands,events).await}
        else{guest(&endpoint,&options,invite.ok_or("Invite is missing")?,commands,events).await}
    };
    let cancelled=async{while !stop.load(Ordering::Relaxed){tokio::time::sleep(Duration::from_millis(50)).await;}};
    let result=tokio::select!{result=work=>result,_=cancelled=>Ok(())};
    let _=tokio::time::timeout(Duration::from_secs(2),endpoint.close()).await;
    result
}

enum Inbound{
    Join{hello:Wire,tx:channel::Sender<Wire>,reply:oneshot::Sender<Result<(u8,u64,Room),String>>},
    Message(u8,u64,Wire),Left(u8,u64),
}
struct Peer{slot:u8,generation:u64,tx:channel::Sender<Wire>}
fn match_event(events:&SyncSender<Event>,message:&Wire)->Result<(),String>{
    let event=match message{Wire::Prepare(start)=>Event::Prepare(start.clone()),Wire::Running=>Event::Running,Wire::Frame(frame)=>Event::Frame(frame.clone()),_=>return Ok(())};
    events.try_send(event).map_err(|_|"Simulation consumer stopped reading; match closed".into())
}
fn publish_match(peers:&[Peer],events:&SyncSender<Event>,message:Wire)->Result<(),String>{
    for peer in peers{peer.tx.try_send(message.clone()).map_err(|_|"A player stopped receiving match data".to_owned())?;}
    match_event(events,&message)
}
fn game_message(flow:&mut match_flow::Flow,slot:u8,message:Wire)->Result<Option<Wire>,String>{
    match message{
        Wire::Loaded(digest)=>Ok(flow.loaded(slot,digest)?.then_some(Wire::Running)),
        Wire::Input{sequence,action}=>{flow.input(slot,sequence,action)?;Ok(None)},
        Wire::Ack{frame,digest}=>{flow.ack(slot,frame,digest)?;Ok(None)},
        _=>Err("Invalid match request".into()),
    }
}
fn same_token(a:&str,b:&str)->bool{a.len()==b.len()&&a.as_bytes().iter().zip(b.as_bytes()).fold(0u8,|n,(a,b)|n|(a^b))==0}

async fn host(endpoint:&Endpoint,options:&Options,settings:Settings,mut commands:channel::Receiver<Command>,events:&SyncSender<Event>)->Result<(),String>{
    let mut state=State::new(settings,&options.name)?;
    let mut bytes=[0;16];getrandom::fill(&mut bytes).map_err(|_|"Secure invite generation failed")?;
    let token=bytes.iter().map(|b|format!("{b:02x}")).collect::<String>();
    let invite=Invite{endpoint:endpoint.id(),token:token.clone()};
    emit(events,Event::Hosting{invite:invite.encode()});emit(events,Event::Joined{slot:0});emit(events,Event::Room(state.room.clone()));
    let (incoming,mut inbox)=channel::channel(128);let mut tasks=tokio::task::JoinSet::new();
    let mut peers=Vec::<Peer>::new();let mut generation=0;
    let mut flow=match_flow::Flow::default();
    let mut watchdog=tokio::time::interval(Duration::from_secs(1));let mut waiting_since=None;
    loop{
        let before=state.room.revision;
        let progress=flow.progress;
        tokio::select!{
            command=commands.recv()=>match command{
                Some(Command::Mods(mods))=>{if flow.start.is_some(){emit(events,Event::Error("Match mods are locked".into()));}else if let Err(e)=state.mods(0,mods){emit(events,Event::Error(e));}},
                Some(Command::Settings(settings))=>{if flow.start.is_some(){emit(events,Event::Error("Match settings are locked".into()));}else if let Err(e)=state.settings(0,settings){emit(events,Event::Error(e));}},
                Some(Command::Ready(ready))=>{if flow.start.is_none(){state.ready(0,ready)?;}},
                Some(Command::Start)=>{match flow.begin(&state.room){Ok(start)=>publish_match(&peers,events,Wire::Prepare(start))?,Err(e)=>emit(events,Event::Error(e))}},
                Some(Command::Advance)=>{if let Some(frame)=flow.advance()?{publish_match(&peers,events,Wire::Frame(frame))?;}},
                Some(Command::Loaded(digest))=>{if let Some(w)=game_message(&mut flow,0,Wire::Loaded(digest))?{publish_match(&peers,events,w)?;}},
                Some(Command::Input{sequence,action})=>{game_message(&mut flow,0,Wire::Input{sequence,action})?;},
                Some(Command::Ack{frame,digest})=>{game_message(&mut flow,0,Wire::Ack{frame,digest})?;},
                Some(Command::Leave)|None=>break,
            },
            incoming_connection=endpoint.accept()=>{
                let Some(connection)=incoming_connection else{break};
                // Bound unauthenticated handshake work as well as the room's player count.
                if tasks.len()>=8{drop(connection);continue;}
                let incoming=incoming.clone();
                tasks.spawn(async move{
                    if let Ok(Ok(conn))=tokio::time::timeout(Duration::from_secs(8),connection).await{
                        serve_peer(conn,incoming).await;
                    }
                });
            },
            message=inbox.recv()=>match message{
                Some(Inbound::Join{hello,tx,reply})=>{
                    let result=match hello{
                        Wire::Hello{version,token:provided,identity,name}=>{
                            if version!=VERSION{Err("Client protocol versions differ".into())}
                            else if !same_token(&token,&provided){Err("Invite is invalid or has expired".into())}
                            else if identity!=options.identity{Err("Game builds or extracted data differ".into())}
                            else if flow.start.is_some(){Err("Match is already in progress".into())}
                            else{state.join(&name).map(|slot|{generation+=1;peers.push(Peer{slot,generation,tx});(slot,generation,state.room.clone())})}
                        },_=>Err("Invalid join request".into()),
                    };
                    let _=reply.send(result);
                },
                Some(Inbound::Message(slot,id,message))=>{
                    if let Some(peer)=peers.iter().find(|p|p.slot==slot&&p.generation==id){
                        match message{
                            Wire::Ready(ready)=>{if flow.start.is_none(){let _=state.ready(slot,ready);}},
                            Wire::Ping(n)=>{let _=peer.tx.try_send(Wire::Pong(n));},
                            message @ (Wire::Loaded(_)|Wire::Input{..}|Wire::Ack{..})=>{if let Some(w)=game_message(&mut flow,slot,message)?{publish_match(&peers,events,w)?;}},
                            _=>{let _=peer.tx.try_send(Wire::Reject("Only the host can change room settings".into()));},
                        }
                    }
                },
                Some(Inbound::Left(slot,id))=>{
                    if peers.iter().any(|p|p.slot==slot&&p.generation==id){
                        if flow.start.is_some(){return Err("A player disconnected; match stopped".into());}
                        peers.retain(|p|p.slot!=slot);state.leave(slot);
                    }
                },None=>break,
            },
            _=tasks.join_next(),if !tasks.is_empty()=>{},
            _=watchdog.tick()=>{},
        }
        if flow.waiting(){
            if progress!=flow.progress||waiting_since.is_none(){waiting_since=Some(Instant::now());}
            if waiting_since.is_some_and(|t:Instant|t.elapsed()>Duration::from_secs(30)){return Err("A player stopped loading or acknowledging simulation frames".into());}
        }else{waiting_since=None;}
        if before!=state.room.revision{
            // Pruning changes the room too. Publish that revision to survivors now,
            // rather than leaving them with ghost players until another action.
            loop{
                let mut slow=Vec::new();
                for peer in &peers{if peer.tx.try_send(Wire::Room(state.room.clone())).is_err(){slow.push(peer.slot);}}
                if slow.is_empty(){break;}
                for slot in slow{peers.retain(|p|p.slot!=slot);state.leave(slot);}
            }
            emit(events,Event::Room(state.room.clone()));
        }
    }
    tasks.abort_all();emit(events,Event::Disconnected("Room closed".into()));Ok(())
}

async fn serve_peer(conn:Connection,incoming:channel::Sender<Inbound>){
    let result=async{
        let (mut send,mut recv)=tokio::time::timeout(Duration::from_secs(8),conn.accept_bi()).await.map_err(error)?.map_err(error)?;
        let hello=tokio::time::timeout(Duration::from_secs(8),read(&mut recv)).await.map_err(error)??;
        let (tx,mut messages)=channel::channel(32);let (reply,answer)=oneshot::channel();
        incoming.send(Inbound::Join{hello,tx,reply}).await.map_err(error)?;
        let accepted=answer.await.map_err(error)?;
        let (slot,generation,room)=match accepted{
            Ok(v)=>v,Err(e)=>{
                write(&mut send,&Wire::Reject(e)).await?;send.finish().map_err(error)?;
                let _=tokio::time::timeout(Duration::from_secs(2),send.stopped()).await;
                return Ok::<(),String>(());
            },
        };
        if write(&mut send,&Wire::Welcome{version:VERSION,slot,room}).await.is_err(){let _=incoming.send(Inbound::Left(slot,generation)).await;return Ok(());}
        let reader=async{
            let mut window=Instant::now();let mut count=0;
            loop{
                let message=read(&mut recv).await?;
                if window.elapsed()>Duration::from_secs(1){window=Instant::now();count=0;}
                count+=1;if count>64{return Err("Too many room requests".into());}
                incoming.send(Inbound::Message(slot,generation,message)).await.map_err(error)?;
            }
            #[allow(unreachable_code)] Ok::<(),String>(())
        };
        let writer=async{while let Some(message)=messages.recv().await{write(&mut send,&message).await?;}Ok::<(),String>(())};
        tokio::select!{_=reader=>{},_=writer=>{},_=conn.closed()=>{}}
        let _=incoming.send(Inbound::Left(slot,generation)).await;
        Ok(())
    }.await;
    let _=result;conn.close(0u32.into(),b"room connection closed");
}

async fn guest(endpoint:&Endpoint,options:&Options,invite:Invite,mut commands:channel::Receiver<Command>,events:&SyncSender<Event>)->Result<(),String>{
    let conn=tokio::time::timeout(Duration::from_secs(25),endpoint.connect(invite.endpoint,ALPN)).await.map_err(|_|"Could not reach this room. The invite may have expired.".to_owned())?.map_err(|_|"Could not connect to the host".to_owned())?;
    let (mut send,mut recv)=conn.open_bi().await.map_err(error)?;
    write(&mut send,&Wire::Hello{version:VERSION,token:invite.token,identity:options.identity.clone(),name:name(&options.name)}).await?;
    let welcome=tokio::time::timeout(Duration::from_secs(12),read(&mut recv)).await.map_err(|_|"Host did not answer the join request".to_owned())??;
    let mut current_room=match welcome{
        Wire::Welcome{version,slot,room} if version==VERSION&&slot>0&&slot<MAX_PLAYERS as u8&&room.valid()&&room.players.iter().any(|p|p.slot==slot)=>{emit(events,Event::Joined{slot});emit(events,Event::Room(room.clone()));room},
        Wire::Reject(reason)=>return Err(reason),_=>return Err("Invalid room response".into()),
    };
    let (tx,mut inbox)=channel::channel(64);
    let receiver=async{loop{tx.send(read(&mut recv).await?).await.map_err(error)?;}#[allow(unreachable_code)]Ok::<(),String>(())};
    tokio::pin!(receiver);
    let start=Instant::now();let mut ping=tokio::time::interval(Duration::from_secs(2));
    let mut prepared=false;let mut running=false;let mut next_frame=1;let mut input_sequences=[0u64;MAX_PLAYERS];
    loop{tokio::select!{
        result=&mut receiver=>{let _=result;emit(events,Event::Disconnected("Host disconnected".into()));break;},
        command=commands.recv()=>match command{
            Some(Command::Ready(ready))=>write(&mut send,&Wire::Ready(ready)).await?,
            Some(Command::Settings(_)|Command::Mods(_))=>emit(events,Event::Error("Only the host can change the room".into())),
            Some(Command::Start|Command::Advance)=>emit(events,Event::Error("Only the host can start or advance the match".into())),
            Some(Command::Loaded(digest))=>write(&mut send,&Wire::Loaded(digest)).await?,
            Some(Command::Input{sequence,action})=>write(&mut send,&Wire::Input{sequence,action}).await?,
            Some(Command::Ack{frame,digest})=>write(&mut send,&Wire::Ack{frame,digest}).await?,
            Some(Command::Leave)|None=>break,
        },
        message=inbox.recv()=>match message{
            Some(Wire::Room(room))=>{
                if prepared||!room.valid()||room.revision<current_room.revision{return Err("Invalid room update".into());}
                current_room=room.clone();emit(events,Event::Room(room));
            },
            Some(message @ Wire::Prepare(_))=>{
                let Wire::Prepare(ref setup)=message else{unreachable!()};
                if prepared||setup.room!=current_room||setup.steps_per_frame!=STEPS_PER_FRAME||setup.seed<=0||setup.seed==i32::MAX||setup.room.players.iter().any(|p|!p.ready)||setup.room.players.len()<2{return Err("Invalid match start".into());}
                prepared=true;match_event(events,&message)?;
            },
            Some(Wire::Running)=>{if !prepared||running{return Err("Invalid simulation start".into());}running=true;match_event(events,&Wire::Running)?;},
            Some(Wire::Frame(frame))=>{
                if !running||frame.number!=next_frame||!frame.valid(&current_room){return Err("Invalid simulation frame".into());}
                for input in &frame.actions{let seq=&mut input_sequences[input.slot as usize];if input.sequence!=*seq+1{return Err("Invalid input sequence".into());}*seq=input.sequence;}
                next_frame+=1;match_event(events,&Wire::Frame(frame))?;
            },
            Some(Wire::Pong(n))=>emit(events,Event::Latency((start.elapsed().as_millis() as u64).saturating_sub(n).min(u32::MAX as u64) as u32)),
            Some(Wire::Reject(e))=>emit(events,Event::Error(e)),
            _=>{},
        },
        _=ping.tick()=>write(&mut send,&Wire::Ping(start.elapsed().as_millis() as u64)).await?,
    }}
    conn.close(0u32.into(),b"left room");Ok(())
}
