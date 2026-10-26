//! Simulation-level replay checks, separate from transport-only command-history tests.
//! These compare observable combat state; they are not a complete state serializer.
use crate::game::{Game,tests_support};
use rrbtd6_coop::{Action,Frame,Input,STEPS_PER_FRAME};
use serde_json::{Value,json};

fn observations(g:&Game)->Value{
    let towers:Vec<_>=g.towers.iter().map(|t|{
        let mut abilities:Vec<_>=t.ability_cd.iter().map(|(k,v)|(k,v.to_bits())).collect();abilities.sort_unstable_by_key(|(k,_)|*k);
        json!([t.id,t.def,t.pos,t.tiers,t.invested,t.pops,t.shots,t.weapons.iter().map(|w|w.last).collect::<Vec<_>>(),t.bank,t.xp,t.point,abilities])
    }).collect();
    let bloons:Vec<_>=g.bloons.iter().map(|b|json!([b.id,b.kind,b.dist,b.pos,b.hp,b.decamo,b.path,b.base,b.grow_t,b.grow_ceiling])).collect();
    let projectiles:Vec<_>=g.projectiles.iter().map(|p|json!([p.uid,p.owner,p.pos,p.vel,p.frames_left,p.pierce,p.target,p.value])).collect();
    json!([g.cash,g.debt,g.lives,g.round,g.round_active,g.game_over,g.won,g.popped,towers,bloons,projectiles])
}
fn apply(g:&mut Game,frame:&Frame){
    for input in &frame.actions{
        match &input.action{
            Action::Place{kind,at}=>{let def=g.tower_defs.iter().position(|(id,_)|id==kind).unwrap();assert!(g.place(def,*at));},
            Action::Upgrade{tower,path}=>{let t=g.towers.iter().position(|t|t.id==*tower).unwrap();assert!(g.upgrade(t,*path as usize));},
            Action::Sell{tower}=>{let t=g.towers.iter().position(|t|t.id==*tower).unwrap();g.sell(t);},
            Action::StartRound=>g.start_round(),
            _=>panic!("Unsupported replay-test action"),
        }
    }
    for _ in 0..STEPS_PER_FRAME{g.step();}
}

#[test]
fn two_replicas_replay_combat_upgrades_and_stable_tower_ids(){
    let mut a=tests_support::game();let mut b=tests_support::game();
    let dart=a.tower_defs.iter().position(|(id,_)|id=="DartMonkey").unwrap();
    // Use a real valid placement, then reproduce it through a network-shaped action.
    let mut placement=tests_support::game();let t=tests_support::place_near(&mut placement,dart,180.0);let at=placement.towers[t].pos;
    let mut sequence=[0u64;2];
    let mut saw_bloons=false;let mut saw_projectiles=false;
    for number in 1..=1000{
        let action=match number{
            1=>Some((0,Action::Place{kind:"DartMonkey".into(),at})),
            2=>Some((1,Action::StartRound)),
            90=>Some((0,Action::Upgrade{tower:a.towers[0].id,path:0})),
            // Start requests while a round is active should have the same no-op result.
            n if n%80==0=>Some((1,Action::StartRound)),
            900=>Some((0,Action::Sell{tower:a.towers[0].id})),
            _=>None,
        };
        let actions=action.map(|(slot,action)|{sequence[slot]+=1;Input{slot:slot as u8,sequence:sequence[slot],action}}).into_iter().collect();
        let host=Frame{number,actions};
        // Actual wire serialization, not a shared object passed to both replicas.
        let guest:Frame=serde_json::from_slice(&serde_json::to_vec(&host).unwrap()).unwrap();
        apply(&mut a,&host);apply(&mut b,&guest);
        saw_bloons|=!a.bloons.is_empty();saw_projectiles|=!a.projectiles.is_empty();
        assert_eq!(observations(&a),observations(&b),"Replica divergence at frame {number}");
        // Renderers consume effects at different times. Those must not alter combat.
        a.pops.clear();a.sfx.clear();a.tracers.clear();a.bursts.clear();a.cash_pops.clear();
    }
    assert!(saw_bloons&&saw_projectiles&&a.popped>0,"Replay must exercise real combat");
}
