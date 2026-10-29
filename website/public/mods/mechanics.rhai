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
fn on_new_game(ctx) {
    this.bonus=100;
    [#{op:"ui",widget:#{id:"launcher",tree:button("Open","MOD WORKSHOP",0.0,-300.0,580.0,0.5,1.0),
       actions:#{"":"on_open"},modal:false}}]
}
fn on_open(ctx) {
    let panel=#{name:"Workshop",active:true,rect:rect(0.0,0.0,1400.0,1000.0,0.5,0.5),
        comps:[#{type:"Image",enabled:true,color:[0.05,0.18,0.32,0.98]}],children:[
            label("Title","MECHANICS WORKSHOP",360.0),
            label("Reward","Round bonus: $"+this.bonus,200.0),
            button("Wave","SEND 30 BLUE BLOONS",0.0,20.0,1100.0,0.5,0.5),
            button("Bonus","INCREASE ROUND BONUS",0.0,-160.0,1100.0,0.5,0.5),
            button("Close","CLOSE",0.0,-350.0,600.0,0.5,0.5)]};
    [#{op:"ui",widget:#{id:"panel",tree:panel,actions:#{"Wave":"on_wave","Bonus":"on_bonus","Close":"on_close"},modal:true}}]
}
fn on_wave(ctx) {
    [#{op:"spawn",name:"Blue",count:30,spacing:0.2},#{op:"popup",title:"Custom wave",body:"30 Blue Bloons queued using native spawning."}]
}
fn on_bonus(ctx) {
    this.bonus+=50;
    this.on_open(ctx)
}
fn on_round_end(ctx) {
    if ctx.sandbox { return []; }
    [#{op:"cash",amount:this.bonus}]
}
fn on_close(ctx) { [#{op:"remove_ui",id:"panel"}] }
