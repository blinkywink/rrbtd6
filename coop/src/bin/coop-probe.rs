//! Headless two-process transport verification. Writes temporary invites only to the requested folder.
use rrbtd6_coop::{Client,Command,Event,Invite,Options,Settings,Action};
use std::{path::PathBuf,time::{Duration,Instant}};
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(1);}}
fn run()->Result<(),String>{
    let args:Vec<String>=std::env::args().collect();
    let role=args.get(1).ok_or("Expected host/join and output directory")?;
    let players=args.iter().find_map(|a|a.strip_prefix("--players=")).unwrap_or("2").parse::<usize>().map_err(|_|"Invalid player count")?;
    if !(2..=4).contains(&players){return Err("Player count must be 2 to 4".into());}
    let mut own_slot=0u8;
    let dir=PathBuf::from(args.get(2).ok_or("Expected output directory")?);std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    let mut options=Options::new(if role=="host"{"Probe Host"}else{"Probe Guest"},if args.iter().any(|a|a=="--mismatch"){ "wrong-build"}else{"probe-1"});
    options.force_relay=args.iter().any(|a|a=="--relay-only");
    let client=if role=="host"{Client::host(options,Settings::default())}else{
        let mut code=std::fs::read_to_string(dir.join("invite.txt")).map_err(|e|e.to_string())?;
        if args.iter().any(|a|a=="--wrong-token"){
            let last=code.pop().ok_or("Empty code")?;code.push(if last=='0'{'1'}else{'0'});
        }
        Client::join(options,Invite::parse(&code)?)
    };
    let start=Instant::now();let mut changed=false;let mut ready=false;let mut ready_at=None;let mut rtt=None;
    let mods_test=args.iter().any(|a|a=="--mods");let mut mods_sent=false;
    let match_test=args.iter().any(|a|a=="--match");let mut started=false;let mut running=false;
    let desync=args.iter().any(|a|a=="--desync");
    let mut frames=0u64;let mut checksum=[7u8;32];let mut advance_at=Instant::now();
    loop{
        for event in client.poll(){match event{
            Event::Hosting{invite}=>std::fs::write(dir.join("invite.txt"),invite).map_err(|e|e.to_string())?,
            Event::Joined{slot}=>own_slot=slot,
            Event::Room(room)=>{
                if role=="host"{
                    if mods_test&&!mods_sent{
                        let package=rrbtd6_coop::ModPackage::new(r#"{"api_version":1,"id":"relay-test","name":"Relay test","version":"1","content":[{"op":"set","path":"/modes/Medium/cash","value":1234}]}"#.into())?;
                        client.send(Command::Mods(vec![package]))?;mods_sent=true;
                    }
                    if room.players.len()==players&&!changed{
                        let mut settings=room.settings.clone();settings.map="MonkeyMeadow".into();settings.difficulty=2;
                        client.send(Command::Settings(settings))?;changed=true;
                    }
                    ready|=room.players.len()==players&&room.players.iter().filter(|p|p.slot!=0).all(|p|p.ready);
                    if match_test&&ready&&!started{
                        if !room.players.iter().any(|p|p.slot==0&&p.ready){client.send(Command::Ready(true))?;}
                        else{client.send(Command::Start)?;started=true;}
                    }
                    if !match_test&&ready&&room.players.len()==1{
                        std::fs::write(dir.join("host-result.json"),r#"{"connected":true,"settings":true,"ready":true,"disconnect":true}"#).map_err(|e|e.to_string())?;
                        let _=client.send(Command::Leave);return Ok(());
                    }
                }else{
                    if mods_test&&room.settings.map=="MonkeyMeadow"{
                        if room.mods.len()!=1||room.mods[0].id!="relay-test"{return Err("Host mod did not transfer to guest".into());}
                        rrbtd6_coop::mods::validate(&room.mods)?;
                    }
                    if room.settings.map=="MonkeyMeadow"&&room.settings.difficulty==2&&!changed{client.send(Command::Ready(true))?;changed=true;}
                    if room.players.iter().any(|p|p.slot==own_slot&&p.ready){ready=true;ready_at.get_or_insert(Instant::now());}
                }
            },
            Event::Latency(ms)=>rtt=Some(ms),
            Event::Prepare(setup)=>{
                if !match_test||setup.room.settings.map!="MonkeyMeadow"{return Err("Unexpected match configuration".into());}
                if mods_test&&(setup.room.mods.len()!=1||setup.room.mods[0].id!="relay-test"){return Err("Match lost selected mods".into());}
                client.send(Command::Loaded(if desync&&role!="host"{[8;32]}else{checksum}))?;
            },
            Event::Running=>{
                running=true;
                client.send(Command::Input{sequence:1,action:if role=="host"{Action::Place{kind:"DartMonkey".into(),at:[12.0,20.0]}}else{Action::StartRound}})?;
            },
            Event::Frame(frame)=>{
                if frame.number!=frames+1{return Err("Frame order differs".into());}
                let mut hash=blake3::Hasher::new();hash.update(&checksum);hash.update(&serde_json::to_vec(&frame).map_err(|e|e.to_string())?);checksum=*hash.finalize().as_bytes();
                frames=frame.number;client.send(Command::Ack{frame:frames,digest:checksum})?;
                if frames==120{
                    let body=serde_json::json!({"frames":frames,"digest":checksum,"settings":true});
                    std::fs::write(dir.join(if role=="host"{"host-match.json"}else{"join-match.json"}),body.to_string()).map_err(|e|e.to_string())?;
                    if role!="host"{std::thread::sleep(Duration::from_millis(300));client.send(Command::Leave)?;std::thread::sleep(Duration::from_millis(250));return Ok(());}
                }
            },
            Event::Error(e)=>{
                if desync&&role=="host"&&e.contains("Initial simulation states differ"){
                    std::fs::write(dir.join("host-desync.json"),r#"{"stopped":true}"#).map_err(|e|e.to_string())?;return Ok(());
                }
                if match_test&&role=="host"&&frames==120&&e.contains("disconnected"){return Ok(());}
                if args.iter().any(|a|a=="--expect-reject"){
                    let expected=if args.iter().any(|a|a=="--wrong-token"){"Invite is invalid"}else{"builds or extracted data differ"};
                    if !e.contains(expected){return Err(format!("Expected specific rejection, got: {e}"));}
                    let body=serde_json::json!({"rejected":true,"reason":e});
                    std::fs::write(dir.join("reject-result.json"),body.to_string()).map_err(|e|e.to_string())?;return Ok(());
                }
                return Err(e);
            },
            Event::Disconnected(e)=>{
                if desync&&role!="host"&&!running{
                    std::fs::write(dir.join("join-desync.json"),r#"{"stopped":true}"#).map_err(|e|e.to_string())?;return Ok(());
                }
                return Err(e);
            },
        }}
        if match_test&&role=="host"&&running&&frames<120&&advance_at.elapsed()>Duration::from_millis(100){client.send(Command::Advance)?;advance_at=Instant::now();}
        if !match_test&&role!="host"&&ready&&ready_at.is_some_and(|t|t.elapsed()>Duration::from_secs(3)){
            let body=serde_json::json!({"connected":true,"settings":changed,"ready":ready,"rttMs":rtt});
            std::fs::write(dir.join("join-result.json"),body.to_string()).map_err(|e|e.to_string())?;
            client.send(Command::Leave)?;std::thread::sleep(Duration::from_millis(250));return Ok(());
        }
        if start.elapsed()>Duration::from_secs(65){return Err("Transport verification timed out".into());}
        std::thread::sleep(Duration::from_millis(10));
    }
}
