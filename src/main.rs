#[macro_use]
extern crate glium;
extern crate image;
extern crate nalgebra;

mod steve;

const VERT_PROG: &'static str = include_str!("vert.glsl");

const FRAG_PROG: &'static str = include_str!("frag.glsl");

use glium::{Surface, VertexBuffer, Frame, Program};
use glium::index::NoIndices;
use glium::texture::srgb_texture2d::SrgbTexture2d;
use glium::uniforms::MagnifySamplerFilter;
use glium::draw_parameters::DepthTest;
use glium::backend::glutin_backend::GlutinFacade;
use glium::index::PrimitiveType;
use glium::glutin::{Event, ElementState, VirtualKeyCode};
use std::f32::consts::{FRAC_PI_2, PI};
use std::thread::sleep_ms;
use nalgebra::{Rot3, Iso3, Vec3, Persp3, ToHomogeneous};

enum NextAction {
    Quit,
}

fn draw_frame(target: &mut Frame, vertex_buffer: &VertexBuffer<steve::Vertex>, index_buffer: &NoIndices, shader_prog: &Program, texture: &SrgbTexture2d, t: f32) {
    target.clear_color_and_depth((0.0, 0.0, 1.0, 1.0), 1.0);

    let perspective = {
        let (width, height) = target.get_dimensions();
        let aspect_ratio = width as f32 / height as f32;
        //println!("Aspect ratio: {} ({} x {})", aspect_ratio, height, width);

        let fov: f32 = 3.141592 / 3.0;
        let zfar = 1024.0;
        let znear = 0.1;

        let pmat = Persp3::new(aspect_ratio, fov, znear, zfar).to_mat();
        pmat.as_array().clone()
    };

    let rot1 = Rot3::new(Vec3::new(-FRAC_PI_2, 0.0, 0.0));
    let rot2 = Rot3::new(Vec3::new(0.0, FRAC_PI_2, 0.0));
    let rot3 = Rot3::new(Vec3::new(0.0, t, 0.0));
    let model = Iso3::new_with_rotmat(Vec3::new(0.0, 10.0, 100.0), rot3 * rot2 * rot1)
        .to_homogeneous().as_array().clone();

    let uniforms = uniform!{
        model: model,
        view:  [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0f32],
            ],
        projection:  perspective,
        tex: texture.sampled().magnify_filter(MagnifySamplerFilter::Nearest),
    };

    let params = glium::DrawParameters {
        depth: glium::Depth {
            test: DepthTest::IfLess,
            write: true,
            .. Default::default()
        },
        .. Default::default()
    };

    target.draw(vertex_buffer, index_buffer, shader_prog, &uniforms, &params).unwrap();
}

fn handle_input(turn_rate: &mut f32, state: ElementState, key_char: u8, vk_opt: &Option<VirtualKeyCode>) -> Option<NextAction> {
    let mut next_action = None;
    let new_turn_rate = match *vk_opt {
        Some(vk) => match (vk, state) {
            (VirtualKeyCode::D, ElementState::Pressed)  => PI / 200.0,
            (VirtualKeyCode::A, ElementState::Pressed)  => -PI / 200.0,
            (VirtualKeyCode::D, ElementState::Released) => 0.0f32,
            (VirtualKeyCode::A, ElementState::Released) => 0.0f32,
            (VirtualKeyCode::Q, ElementState::Released) => { next_action =  Some(NextAction::Quit); 0.0f32},
            _ => 0.0f32
        },
        None => 0.0f32
    };
    *turn_rate = new_turn_rate;
    next_action
}

fn mainloop(display: &GlutinFacade) {
    use std::io::Cursor;

    let vertex_buffer = VertexBuffer::new(display, &steve::VERTICES).unwrap();
    let index_buffer = NoIndices(PrimitiveType::TrianglesList);
    let shader_prog = Program::from_source(display, VERT_PROG, FRAG_PROG, None).unwrap();

    let image = image::load(Cursor::new(&include_bytes!("steve.png")[..]),
                            image::PNG).unwrap();
    let texture = SrgbTexture2d::new(display, image).unwrap();

    let mut t = 0.0f32;
    let mut turn_rate = 0.0f32;

    loop {
        t += turn_rate;

        for ev in display.poll_events() {
            match ev {
                Event::Closed => return,
                Event::KeyboardInput(state, key_char, vk_opt) => match handle_input(&mut turn_rate, state, key_char, &vk_opt) {
                    Some(NextAction::Quit) => return,
                    None => ()
                },
                _ => ()
            }
        }

        let (width, height) = display.get_framebuffer_dimensions();
        //println!("Framebuffer dimensions: {} x {}", width, height);
        let mut target = display.draw();
        draw_frame(&mut target, &vertex_buffer, &index_buffer, &shader_prog, &texture, t);
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
