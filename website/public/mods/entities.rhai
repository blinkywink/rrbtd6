// This is executed by the embedded engine, not Python or an injected DLL.
// Each event gets a small read-only game snapshot; `this` is persistent match state.
fn rect(x,y,w,h,ax,ay) {
    #{anchor_min:[ax,ay],anchor_max:[ax,ay],pos:[x,y],size:[w,h],pivot:[0.5,0.5],scale:[1.0,1.0],rot_z:0.0}
}
fn label(name,text,y) {
    #{name:name,active:true,rect:rect(0.0,y,900.0,100.0,0.5,0.5),
      comps:[#{type:"TextMeshProUGUI",enabled:true,m_text:text,m_fontSize:60.0,color:[1.0,1.0,1.0,1.0]}],children:[]}
}
fn button(name,text,x,y,w,ax,ay) {
    #{name:name,active:true,rect:rect(x,y,w,145.0,ax,ay),
      comps:[#{type:"Image",enabled:true,sprite:"GreenBtnLong",m_Type:1,color:[1.0,1.0,1.0,1.0]},#{type:"Button",enabled:true}],
      children:[label("Caption",text,0.0)]}
}
fn flag(value) {if value {return "ON";} "OFF"}
fn on_new_game(ctx) {
    this.boost=false;this.shield=false;this.block_farms=false;this.charges=5;this.panel=false;
    [#{op:"ui",widget:#{id:"launcher",tree:button("Open","ENTITY LAB",-650.0,-300.0,550.0,0.5,1.0),actions:#{"":"on_open"},modal:false}}]
}
fn on_open(ctx) {
    this.panel=true;
    let tree=#{name:"Lab",active:true,rect:rect(0.0,0.0,1600.0,1550.0,0.5,0.5),
        comps:[#{type:"Image",enabled:true,sprite:"MainBGPanelBlue",m_Type:1,color:[1.0,1.0,1.0,1.0]}],children:[
            label("Title","ENTITY LAB",650.0),
            button("Inspect","INSPECT TOWERS",0.0,460.0,1200.0,0.5,0.5),
            button("Heal","ADD 50 HP TO FIRST BLOON",0.0,270.0,1300.0,0.5,0.5),
            button("Boost","DOUBLE SHOT DAMAGE: "+flag(this.boost),0.0,80.0,1300.0,0.5,0.5),
            button("Shield","ABSORB HITS: "+flag(this.shield),0.0,-110.0,1300.0,0.5,0.5),
            button("Rule","BLOCK FARM PURCHASES: "+flag(this.block_farms),0.0,-300.0,1300.0,0.5,0.5),
            label("Status","Shield charges: "+this.charges,-460.0),
            button("Close","CLOSE",0.0,-630.0,600.0,0.5,0.5)]};
    [#{op:"ui",widget:#{id:"panel",tree:tree,actions:#{Inspect:"on_inspect",Heal:"on_heal",Boost:"on_boost",Shield:"on_shield",Rule:"on_rule",Close:"on_close"},modal:true}}]
}
fn on_inspect(ctx) { [#{op:"inspect",query:#{kind:"towers",offset:0,limit:4,tag:"list"}}] }
fn on_heal(ctx) { [#{op:"inspect",query:#{kind:"bloons",offset:0,limit:1,tag:"heal"}}] }
fn on_query(ctx) {
    if ctx.event.tag=="heal" {
        if ctx.event.items.len==0 {return [#{op:"popup",title:"No Bloons",body:"Send a bloon first."}];}
        let b=ctx.event.items[0];
        return [#{op:"entity_patch",patch:#{kind:"bloons",id:b.id,layer:b.layer,fields:#{hp:b.hp+50}}},
            #{op:"popup",title:"Bloon edited",body:b.bloon+" now has "+(b.hp+50)+" HP."}];
    }
    let body="Live towers: "+ctx.event.total;
    for t in ctx.event.items {body+="\n"+t.tower+" (ID "+t.id+")";}
    [#{op:"popup",title:"Entity snapshot",body:body}]
}
fn on_boost(ctx) { this.boost=!this.boost;this.on_open(ctx) }
fn on_shield(ctx) { this.shield=!this.shield;this.charges=5;this.on_open(ctx) }
fn on_rule(ctx) { this.block_farms=!this.block_farms;this.on_open(ctx) }
fn on_close(ctx) {this.panel=false;[#{op:"remove_ui",id:"panel"}]}
fn on_tick(ctx) {if this.panel {return this.on_open(ctx);} []}
fn before_fire(ctx) {if this.boost && ctx.event.damage>0 {return #{damage:ctx.event.damage*2};} #{}}
fn before_damage(ctx) {
    if this.shield && this.charges>0 && ctx.event.damage>0 {this.charges-=1;return #{cancel:true};} #{}
}
fn before_place(ctx) {#{cancel:this.block_farms && ctx.event.tower=="BananaFarm"}}
