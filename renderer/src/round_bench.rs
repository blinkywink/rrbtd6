//! Repeatable late-game scenes; benchmark setup only, never used by normal play.
use crate::game::Game;

pub fn setup(game:&mut Game,round:usize,degree:u32){
    let count=match round{98=>1,140=>2,200=>4,_=>panic!("unsupported round benchmark")};
    let names=["DartMonkey","BoomerangMonkey","NinjaMonkey","WizardMonkey"];
    game.clear_towers();game.removed.clear();game.clear_bloons();
    game.cash=1e12;game.lives=1e12;game.freeplay_seed=12345;game.start_freeplay();
    for (slot,name) in names[..count].iter().enumerate(){
        let def=game.tower_defs.iter().position(|(n,_)|n==name).expect("benchmark tower missing");
        let mut ids=Vec::new();
        for (path,tiers) in [[5,0,0],[0,5,0],[0,0,5]].into_iter().enumerate(){
            let length=game.track.length();
            let center=game.track.pos(length*(0.25+slot as f32*0.12));
            let mut placed=false;
            for ring in 0..16{for k in 0..48{
                let a=(k as f32+path as f32*16.0)/48.0*std::f32::consts::TAU;
                let r=24.0+ring as f32*6.0;
                if game.place(def,[center[0]+r*a.cos(),center[1]+r*a.sin()]){placed=true;break;}
            }if placed{break;}}
            assert!(placed,"cannot place benchmark {name}");
            let t=game.towers.last_mut().unwrap();t.tiers=tiers;ids.push(t.id);
        }
        let t=game.towers.iter().position(|t|t.id==ids[0]).unwrap();
        assert!(game.buy_paragon(t,0.0),"cannot create benchmark paragon {name}");
        let t=game.towers.iter().position(|t|t.id==ids[0]).unwrap();
        let base=&game.tower_tiers[def].as_ref().unwrap().paragon.as_ref().unwrap().tier;
        game.towers[t].paragon=Some((degree,std::sync::Arc::new(game.paragon_tier(base,degree))));
        eprintln!("round-bench: {name} degree {degree} at {:?}",game.towers[t].pos);
    }
    assert_eq!(game.towers.iter().filter(|t|t.paragon.is_some()).count(),count);
    game.round=round-1;game.round_active=false;game.won=false;game.game_over=false;
    game.place_fx.clear();game.upgrade_fx.clear();
    eprintln!("round-bench: round {round}, {count} paragons, degree {degree}, freeplay seed 12345; no abilities automated");
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn presets_have_exact_round_degree_and_paragon_counts(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let json=std::fs::read_to_string(root.join("local_extracted/game/game.json")).unwrap();
        for (round,count) in [(98,1),(140,2),(200,4)]{
            let mut g=Game::new(serde_json::from_str(&json).unwrap());setup(&mut g,round,20);
            assert_eq!(g.round+1,round);assert_eq!(g.freeplay_seed,12345);assert!(g.freeplay);
            assert_eq!(g.towers.len(),count);assert!(g.towers.iter().all(|t|t.paragon.as_ref().is_some_and(|p|p.0==20)));
            g.start_round();assert!(g.round_active);
        }
    }
}
