//! Immutable map geometry. Raw draws are limited to the untransformed, unclipped
//! world pass; mutable models continue through macroquad's streaming renderer.
use macroquad::{miniquad as mq, prelude::*};
use std::{cell::{Cell, RefCell}, rc::{Rc, Weak}};
use crate::render3d::{Grad, Model};

thread_local! {
    static PROGRAMS:RefCell<Weak<Programs>>=RefCell::new(Weak::new());
    static LIVE_BUFFERS:Cell<usize>=const {Cell::new(0)};
}

pub fn enabled()->bool{
    static ENABLED:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
    *ENABLED.get_or_init(||std::env::var_os("RRBTD6_NO_STATIC_MAP_GPU").is_none()
        &&std::env::var_os("RRBTD6_LEGACY_RENDER_STORAGE").is_none())
}
pub fn live_buffers()->usize{LIVE_BUFFERS.with(Cell::get)}

struct Program{pipeline:mq::Pipeline,shader:mq::ShaderId}
impl Program{
    fn new(vertex:&str,fragment:&str,extra:Vec<mq::UniformDesc>)->Option<Self>{
        let mut uniforms=vec![mq::UniformDesc::new("Model",mq::UniformType::Mat4),mq::UniformDesc::new("Projection",mq::UniformType::Mat4)];
        uniforms.extend(extra);
        let ctx=unsafe{get_internal_gl()}.quad_context;
        let shader=ctx.new_shader(mq::ShaderSource::Glsl{vertex,fragment},mq::ShaderMeta{
            images:vec!["Texture".into()],uniforms:mq::UniformBlockLayout{uniforms},
        }).ok()?;
        let pipeline=ctx.new_pipeline(&[mq::BufferLayout::default()],&[
            mq::VertexAttribute::new("position",mq::VertexFormat::Float3),
            mq::VertexAttribute::new("texcoord",mq::VertexFormat::Float2),
            mq::VertexAttribute::new("color0",mq::VertexFormat::Byte4),
            mq::VertexAttribute::new("normal",mq::VertexFormat::Float4),
        ],shader,mq::PipelineParams{depth_test:mq::Comparison::LessOrEqual,depth_write:true,..Default::default()});
        Some(Self{pipeline,shader})
    }
}
impl Drop for Program{
    fn drop(&mut self){let ctx=unsafe{get_internal_gl()}.quad_context;ctx.delete_pipeline(self.pipeline);ctx.delete_shader(self.shader);}
}
struct Programs{plain:Program,gradient:Program}
impl Programs{
    fn shared()->Option<Rc<Self>>{
        if let Some(p)=PROGRAMS.with(|p|p.borrow().upgrade()){return Some(p);}
        if unsafe{get_internal_gl()}.quad_context.info().backend!=mq::Backend::OpenGl{return None;}
        let plain=Program::new(crate::render3d::SPRITE_VS,crate::render3d::SPRITE_FS,vec![])?;
        let gradient=Program::new(crate::render3d::GRAD_VS,crate::render3d::GRAD_FS,vec![
            mq::UniformDesc::new("Bot",mq::UniformType::Float4),mq::UniformDesc::new("Mid",mq::UniformType::Float4),
            mq::UniformDesc::new("Top",mq::UniformType::Float4),mq::UniformDesc::new("Middle",mq::UniformType::Float1),
        ])?;
        let p=Rc::new(Self{plain,gradient});PROGRAMS.with(|cache|*cache.borrow_mut()=Rc::downgrade(&p));Some(p)
    }
}

#[repr(C)]struct BaseUniforms{model:[f32;16],projection:[f32;16]}
#[repr(C)]struct GradientUniforms{base:BaseUniforms,bot:[f32;4],mid:[f32;4],top:[f32;4],middle:f32}

struct Run{
    sources:Vec<usize>,bindings:mq::Bindings,indices:i32,gradient:Option<Grad>,
    _texture:Texture2D,
}
impl Drop for Run{
    fn drop(&mut self){
        let ctx=unsafe{get_internal_gl()}.quad_context;
        ctx.delete_buffer(self.bindings.vertex_buffers[0]);ctx.delete_buffer(self.bindings.index_buffer);
        LIVE_BUFFERS.with(|n|n.set(n.get()-2));
    }
}
enum Entry{Cached(Run),Stream(usize)}
pub struct Geometry{entries:Vec<Entry>,programs:Rc<Programs>,bytes:usize,runs:usize}
#[derive(Default)]pub struct DrawStats{pub cached_draws:usize,pub streamed_parts:usize}

struct Pending{sources:Vec<usize>,vertices:Vec<Vertex>,indices:Vec<u16>,texture:Option<Texture2D>,gradient:Option<Grad>}
fn same_gradient(a:Option<Grad>,b:Option<Grad>)->bool{
    let bits=|g:Grad|(g.bot.to_array().map(f32::to_bits),g.mid.to_array().map(f32::to_bits),g.top.to_array().map(f32::to_bits),g.middle.to_bits());
    match (a,b){(None,None)=>true,(Some(a),Some(b))=>bits(a)==bits(b),_=>false}
}
impl Pending{
    fn new(i:usize,mesh:&Mesh,gradient:Option<Grad>)->Self{
        Self{sources:vec![i],vertices:mesh.vertices.clone(),indices:mesh.indices.clone(),texture:mesh.texture.clone(),gradient}
    }
    fn accepts(&self,mesh:&Mesh,gradient:Option<Grad>)->bool{
        self.texture==mesh.texture&&same_gradient(self.gradient,gradient)
            &&self.vertices.len()+mesh.vertices.len()<crate::render3d::vertex_capacity()
            &&self.indices.len()+mesh.indices.len()<5000
    }
    fn append(&mut self,i:usize,mesh:&Mesh){
        let base=self.vertices.len() as u16;self.sources.push(i);self.vertices.extend_from_slice(&mesh.vertices);
        self.indices.extend(mesh.indices.iter().map(|&v|base+v));
    }
    fn upload(self)->(Run,usize){
        let texture=self.texture.unwrap_or_else(Texture2D::empty);let ctx=unsafe{get_internal_gl()}.quad_context;
        let vertex=ctx.new_buffer(mq::BufferType::VertexBuffer,mq::BufferUsage::Immutable,mq::BufferSource::slice(&self.vertices));
        let index=ctx.new_buffer(mq::BufferType::IndexBuffer,mq::BufferUsage::Immutable,mq::BufferSource::slice(&self.indices));
        LIVE_BUFFERS.with(|n|n.set(n.get()+2));
        let bytes=self.vertices.len()*std::mem::size_of::<Vertex>()+self.indices.len()*2;
        (Run{sources:self.sources,bindings:mq::Bindings{vertex_buffers:vec![vertex],index_buffer:index,images:vec![texture.raw_miniquad_id()]},
            indices:self.indices.len() as i32,gradient:self.gradient,_texture:texture},bytes)
    }
}

impl Geometry{
    pub async fn build(model:&Model,budget:&mut crate::load_work::Budget)->Option<Self>{
        if !enabled()||!model.fx.iter().any(Option::is_none){return None;}
        let programs=Programs::shared()?;let mut entries=Vec::new();let mut pending:Option<Pending>=None;let mut bytes=0;let mut runs=0;
        for (i,mesh) in model.parts.iter().enumerate(){
            if model.fx[i].as_ref().is_some_and(|f|f.shader.transparent){continue;}
            let stream=model.fx[i].is_some();
            if stream||pending.as_ref().is_some_and(|p|!p.accepts(mesh,model.grads[i])){
                if let Some(p)=pending.take(){let (run,size)=p.upload();bytes+=size;runs+=1;entries.push(Entry::Cached(run));budget.checkpoint().await;}
            }
            if stream{entries.push(Entry::Stream(i));}
            else if let Some(p)=pending.as_mut(){p.append(i,mesh);}
            else{pending=Some(Pending::new(i,mesh,model.grads[i]));}
        }
        if let Some(p)=pending{let (run,size)=p.upload();bytes+=size;runs+=1;entries.push(Entry::Cached(run));budget.checkpoint().await;}
        if runs==0{return None;}
        Some(Self{entries,programs,bytes,runs})
    }
    pub fn resource_sizes(&self)->(usize,usize){(self.runs,self.bytes)}

    /// The caller guarantees identity model transform and no scissor. Pass
    /// viewport explicitly: None means the full current render target, not the
    /// logical window dimensions (important for DPI and render scaling).
    pub fn draw(&self,hidden:&[bool],viewport:Option<(i32,i32,i32,i32)>,mut stream:impl FnMut(usize))->DrawStats{
        let projection=unsafe{get_internal_gl()}.quad_gl.get_projection_matrix();
        let base=||BaseUniforms{model:Mat4::IDENTITY.to_cols_array(),projection:projection.to_cols_array()};
        let mut raw=false;let mut stats=DrawStats::default();
        for entry in &self.entries{
            let draw=match entry{Entry::Cached(run) if run.sources.iter().all(|&i|!hidden[i])=>Some(run),_=>None};
            if let Some(run)=draw{
                if !raw{
                    let mut gl=unsafe{get_internal_gl()};gl.flush();
                    let pass=gl.quad_gl.get_active_render_pass();let ctx=gl.quad_context;
                    let (w,h)=pass.map(|p|ctx.texture_size(ctx.render_pass_texture(p))).unwrap_or_else(||{let (w,h)=mq::window::screen_size();(w as u32,h as u32)});
                    let (x,y,vw,vh)=viewport.unwrap_or((0,0,w as i32,h as i32));
                    ctx.begin_pass(pass,mq::PassAction::Nothing);ctx.apply_viewport(x,y,vw,vh);ctx.apply_scissor_rect(0,0,w as i32,h as i32);raw=true;
                }
                let ctx=unsafe{get_internal_gl()}.quad_context;
                if let Some(g)=run.gradient{
                    ctx.apply_pipeline(&self.programs.gradient.pipeline);ctx.apply_bindings(&run.bindings);
                    ctx.apply_uniforms(mq::UniformsSource::table(&GradientUniforms{base:base(),bot:g.bot.to_array(),mid:g.mid.to_array(),top:g.top.to_array(),middle:g.middle}));
                }else{
                    ctx.apply_pipeline(&self.programs.plain.pipeline);ctx.apply_bindings(&run.bindings);ctx.apply_uniforms(mq::UniformsSource::table(&base()));
                }
                ctx.draw(0,run.indices,1);stats.cached_draws+=1;
            }else{
                if raw{unsafe{get_internal_gl()}.quad_context.end_render_pass();raw=false;}
                match entry{
                    Entry::Stream(i)=>{if !hidden[*i]{stream(*i);stats.streamed_parts+=1;}},
                    Entry::Cached(run)=>for &i in &run.sources{if !hidden[i]{stream(i);stats.streamed_parts+=1;}},
                }
            }
        }
        if raw{unsafe{get_internal_gl()}.quad_context.end_render_pass();}
        stats
    }
}
