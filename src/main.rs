#[macro_use]
extern crate glium;
extern crate image;
extern crate nalgebra;
extern crate num;

mod steve;

const VERT_PROG: &'static str = include_str!("vert.glsl");

const FRAG_PROG: &'static str = include_str!("frag.glsl");

use glium::{Surface, VertexBuffer, Frame, Program};
use glium::index::NoIndices;
use glium::texture::srgb_texture2d::SrgbTexture2d;
use glium::uniforms::{MagnifySamplerFilter, Uniforms, AsUniformValue};
use glium::draw_parameters::DepthTest;
use glium::backend::glutin_backend::GlutinFacade;
use glium::index::PrimitiveType;
use glium::vertex::BufferCreationError;
use glium::glutin::{Event, ElementState, VirtualKeyCode};
use std::f32::consts::{FRAC_PI_2, PI};
use std::thread::sleep_ms;
use nalgebra::{Rot3, Iso3, Vec3, Persp3, ToHomogeneous, Mat4};
use num::traits::{Zero, One};

enum NextAction {
    Quit,
}

fn handle_input(turn_rate_y: &mut f32, turn_rate_x: &mut f32, do_anim: &mut bool, state: ElementState, vk_opt: &Option<VirtualKeyCode>) -> Option<NextAction> {
    let mut next_action = None;
    match *vk_opt {
        Some(vk) => match (vk, state) {
            (VirtualKeyCode::Right, ElementState::Pressed)  => *turn_rate_y = PI / 200.0,
            (VirtualKeyCode::Left, ElementState::Pressed)  => *turn_rate_y = -PI / 200.0,
            (VirtualKeyCode::Right, ElementState::Released) => *turn_rate_y = 0.0f32,
            (VirtualKeyCode::Left, ElementState::Released) => *turn_rate_y = 0.0f32,

            (VirtualKeyCode::Up, ElementState::Pressed)  => *turn_rate_x = PI / 200.0,
            (VirtualKeyCode::Down, ElementState::Pressed)  => *turn_rate_x = -PI / 200.0,
            (VirtualKeyCode::Up, ElementState::Released) => *turn_rate_x = 0.0f32,
            (VirtualKeyCode::Down, ElementState::Released) => *turn_rate_x = 0.0f32,

            (VirtualKeyCode::A, ElementState::Released) => *do_anim = !*do_anim,

            (VirtualKeyCode::Q, ElementState::Released) => next_action =  Some(NextAction::Quit),
            _ => ()
        },
        None => ()
    };
    next_action
}

pub struct ModelPiece {
    vbo: VertexBuffer<steve::Vertex>,
    prim: PrimitiveType,
    bone: Option<Vec3<f32>>,
}

impl ModelPiece {
    fn new(display: &GlutinFacade, verts: &[steve::Vertex], prim: PrimitiveType, bone: Option<Vec3<f32>>) -> Result<Self, BufferCreationError> {
        let vertex_buffer = match VertexBuffer::new(display, verts) {
            Ok(vbo) => vbo,
            Err(e) => return Err(e),
        };
        Ok(ModelPiece{vbo: vertex_buffer, prim: prim, bone: bone})
    }

    fn make_anim_matrix(self: &Self, anim_angle: f32) -> Mat4<f32> {
        match self.bone {
            Some(bone) => {
                let trans1 = Iso3::new(-bone, Vec3::zero()).to_homogeneous();
                let rot3 = Rot3::new(Vec3::new(0.0, anim_angle, 0.0)).to_homogeneous();
                let trans2 = Iso3::new(bone, Vec3::zero()).to_homogeneous();
                trans2 * rot3 * trans1},
            None => Mat4::<f32>::one()
        }
    }

    fn draw<U>(self: &Self, target: &mut Frame, shader_prog: &Program, uniforms: &U, params: &glium::draw_parameters::DrawParameters) where U: Uniforms {
        let ibo = NoIndices(self.prim);
        target.draw(&self.vbo, ibo, shader_prog, uniforms, params).unwrap();
    }
}

macro_rules! implement_uniforms {
    ($struct_name:ident, $($field_name:ident),+) => (
        impl<'b> glium::uniforms::Uniforms for $struct_name<'b> {
            #[inline]
            fn visit_values<'a, F: FnMut(&str, glium::uniforms::UniformValue<'a>)>(&'a self, mut output: F) {
                $(
                    output(stringify!($field_name), self.$field_name.as_uniform_value());
                )+
            }
        }
    );

    ($struct_name:ident, $($field_name:ident),+,) => (
        implement_uniforms!($struct_name, $($field_name),+);
    );
}

#[derive(Copy, Clone)]
struct PlayerModelUniforms<'a> {
    model: Mat4<f32>,
    view:  Mat4<f32>,
    projection:  Mat4<f32>,
    tex: &'a glium::uniforms::Sampler<'a, SrgbTexture2d>,
}

implement_uniforms!(PlayerModelUniforms, model, view, projection, tex);

pub struct PlayerModel {
    head: ModelPiece,
    torso: ModelPiece,

    larm: ModelPiece,
    rarm: ModelPiece,

    lleg: ModelPiece,
    rleg: ModelPiece,

    texture: SrgbTexture2d,
}

impl PlayerModel {
    fn draw(self: &Self, target: &mut Frame, shader_prog: &Program, t: f32, angle_y: f32, angle_x: f32) {
        let perspective = {
            let (width, height) = target.get_dimensions();
            let aspect_ratio = width as f32 / height as f32;
            //println!("Aspect ratio: {} ({} x {})", aspect_ratio, height, width);

            let fov: f32 = 3.141592 / 3.0;
            let zfar = 1024.0;
            let znear = 0.1;

            Persp3::new(aspect_ratio, fov, znear, zfar).to_mat()
        };

        let rot1 = Rot3::new(Vec3::new(-FRAC_PI_2, 0.0, 0.0)).to_homogeneous();
        let rot2 = Rot3::new(Vec3::new(0.0, FRAC_PI_2, 0.0)).to_homogeneous();

        let trans_final = Vec3::new(0.0, 0.0, 100.0);
        let trans_final_mat = Iso3::new(trans_final, Vec3::zero()).to_homogeneous();
        let inv_trans_final_mat = Iso3::new(-trans_final, Vec3::zero()).to_homogeneous();

        let model = trans_final_mat * rot2 * rot1;
        let view_rot1 = Rot3::new(Vec3::new(0.0, angle_y, 0.0)).to_homogeneous();
        let view_rot2 = Rot3::new(Vec3::new(angle_x, 0.0, 0.0)).to_homogeneous();
        let view = trans_final_mat * view_rot2 * view_rot1 * inv_trans_final_mat;

        let mut uniforms = PlayerModelUniforms{
            model: model,
            view: view,
            projection: perspective,
            tex: &self.texture.sampled().magnify_filter(MagnifySamplerFilter::Nearest),
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            .. Default::default()
        };

        self.head.draw(target, shader_prog, &uniforms, &params);
        self.torso.draw(target, shader_prog, &uniforms, &params);

        let anim_matrix = self.larm.make_anim_matrix(-FRAC_PI_2 * t.sin());
        let model = trans_final_mat * rot2 * rot1 * anim_matrix;
        uniforms.model = model;
        self.larm.draw(target, shader_prog, &uniforms, &params);

        let anim_matrix = self.rarm.make_anim_matrix(FRAC_PI_2 * t.sin());
        let model = trans_final_mat * rot2 * rot1 * anim_matrix;
        uniforms.model = model;
        self.rarm.draw(target, shader_prog, &uniforms, &params);

        let anim_matrix = self.lleg.make_anim_matrix(FRAC_PI_2 * t.sin());
        let model = trans_final_mat * rot2 * rot1 * anim_matrix;
        uniforms.model = model;
        self.lleg.draw(target, shader_prog, &uniforms, &params);

        let anim_matrix = self.rleg.make_anim_matrix(-FRAC_PI_2 * t.sin());
        let model = trans_final_mat * rot2 * rot1 * anim_matrix;
        uniforms.model = model;
        self.rleg.draw(target, shader_prog, &uniforms, &params);
    }
}

fn mainloop(display: &GlutinFacade) {
    use std::io::Cursor;

    let image = image::load(Cursor::new(&include_bytes!("steve.png")[..]),
                            image::PNG).unwrap();

    let player = PlayerModel{
        head: ModelPiece::new(display, &steve::HEAD, PrimitiveType::TrianglesList, None).unwrap(),
        torso: ModelPiece::new(display, &steve::TORSO, PrimitiveType::TrianglesList, None).unwrap(),

        larm: ModelPiece::new(display, &steve::LARM, PrimitiveType::TrianglesList, Some(*steve::LARM_BONE)).unwrap(),
        rarm: ModelPiece::new(display, &steve::RARM, PrimitiveType::TrianglesList, Some(*steve::RARM_BONE)).unwrap(),

        lleg: ModelPiece::new(display, &steve::LLEG, PrimitiveType::TrianglesList, Some(*steve::LLEG_BONE)).unwrap(),
        rleg: ModelPiece::new(display, &steve::RLEG, PrimitiveType::TrianglesList, Some(*steve::RLEG_BONE)).unwrap(),

        texture: SrgbTexture2d::new(display, image).unwrap(),
    };

    let shader_prog = Program::from_source(display, VERT_PROG, FRAG_PROG, None).unwrap();

    let mut t = 0.0f32;
    let mut angle_y = 0.0f32;
    let mut angle_x = 0.0f32;

    let anim_rate = 0.04f32;
    let mut turn_rate_y = 0.0f32;
    let mut turn_rate_x = 0.0f32;

    let mut do_anim = true;

    loop {
        if do_anim {
            t += anim_rate;
        }
        angle_y += turn_rate_y;
        angle_x += turn_rate_x;

        for ev in display.poll_events() {
            match ev {
                Event::Closed => return,
                Event::KeyboardInput(state, _, vk_opt) => match handle_input(&mut turn_rate_y, &mut turn_rate_x, &mut do_anim, state, &vk_opt) {
                    Some(NextAction::Quit) => return,
                    None => ()
                },
                _ => ()
            }
        }

        //let (width, height) = display.get_framebuffer_dimensions();
        //println!("Framebuffer dimensions: {} x {}", width, height);
        let mut target = display.draw();
        target.clear_color_and_depth((0.0, 0.0, 1.0, 1.0), 1.0);

        player.draw(&mut target, &shader_prog, t, angle_y, angle_x);

        target.finish().unwrap();
        sleep_ms(16);
    }
}

fn main() {
    use glium::{DisplayBuild, GliumCreationError};
    use glium::glutin::{WindowBuilder, GlRequest, Api, GlProfile};
    let display_option = WindowBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
        .with_gl_profile(GlProfile::Core)
        .with_depth_buffer(24)
        .with_vsync()
        .build_glium();
    println!("If there was a message about an error just now, ignore it.  I think the driver's on crack.");
    match display_option {
        Ok(display) => mainloop(&display),
        Err(creation_error) => match creation_error {
            GliumCreationError::BackendCreationError(_) => println!("Oh, crap!"),
            GliumCreationError::IncompatibleOpenGl(msg) => println!("Incompatible OpenGL: {}", msg)
        }
    }
}
