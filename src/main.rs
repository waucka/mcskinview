#[macro_use]
extern crate glium;
extern crate image;
extern crate nalgebra;
extern crate num;
extern crate getopts;
extern crate inotify;

mod steve_common;
mod steve;
mod steve17;

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
use glium::glutin::{Event, ElementState, VirtualKeyCode, MouseButton};
use std::f32::consts::{FRAC_PI_2, PI};
use std::thread::sleep_ms;
use nalgebra::{Rot3, Iso3, Vec3, Persp3, ToHomogeneous, Mat4};
use num::traits::{Zero, One};
use getopts::Options;
use inotify::INotify;
use inotify::wrapper::Watch;
use inotify::ffi::*;
use std::path::Path;
use std::env;

enum NextAction {
    Reload,
    Quit,
}

fn handle_input(turn_rate_y: &mut f32, turn_rate_x: &mut f32, do_anim: &mut bool, t: &mut f32, state: ElementState, vk_opt: &Option<VirtualKeyCode>) -> Option<NextAction> {
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
            (VirtualKeyCode::R, ElementState::Released) => *t = 0.0f32,

            (VirtualKeyCode::F5, ElementState::Pressed) => next_action = Some(NextAction::Reload),
            (VirtualKeyCode::Q, ElementState::Released) => next_action = Some(NextAction::Quit),
            _ => ()
        },
        None => ()
    };
    next_action
}

struct MouseState {
    left_pressed: bool,
    position: Option<(i32, i32)>,
}

fn handle_mouse_button(button: MouseButton, state: ElementState, mouse_state: &mut MouseState) {
    match (button, state) {
        (MouseButton::Left, ElementState::Pressed) => mouse_state.left_pressed = true,
        (MouseButton::Left, ElementState::Released) => {
            mouse_state.left_pressed = false;
            mouse_state.position = None;
        },
        _ => ()
    }
}

fn handle_mouse_motion(position: (i32, i32), mouse_state: &mut MouseState, angle_y: &mut f32, angle_x: &mut f32) {
    let (nx, ny) = position;
    match (mouse_state.left_pressed, mouse_state.position) {
        (true, Some((x, y))) => {
            let (dx, dy) = (nx - x, ny - y);
            *angle_y += dx as f32 / 100.0;
            *angle_x -= dy as f32 / 100.0;
            mouse_state.position = Some((nx, ny));
        },
        (true, None) => mouse_state.position = Some((nx, ny)),
        _ => ()
    }
}

pub struct ModelPiece {
    vbo: VertexBuffer<steve_common::Vertex>,
    prim: PrimitiveType,
    bone: Option<Vec3<f32>>,
}

impl ModelPiece {
    fn new(display: &GlutinFacade, verts: &[steve_common::Vertex], prim: PrimitiveType, bone: Option<Vec3<f32>>) -> Result<Self, BufferCreationError> {
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
    texture_watch: Option<Watch>,
}

impl PlayerModel {
    fn draw(self: &Self, target: &mut Frame, shader_prog: &Program, t: f32, angle_y: f32, angle_x: f32) {
        use nalgebra::Inv;
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

        let trans_final_mat = Iso3::new(Vec3::new(0.0, 16.0, 100.0), Vec3::zero()).to_homogeneous();
        let model = trans_final_mat * rot2 * rot1;

        let view_center_mat = Iso3::new(Vec3::new(0.0, 0.0, 100.0), Vec3::zero()).to_homogeneous();
        let inv_view_center_mat = view_center_mat.inv().unwrap();
        let view_rot1 = Rot3::new(Vec3::new(0.0, angle_y, 0.0)).to_homogeneous();
        let view_rot2 = Rot3::new(Vec3::new(angle_x, 0.0, 0.0)).to_homogeneous();
        let view = view_center_mat * view_rot2 * view_rot1 * inv_view_center_mat;

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
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
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

fn load_default_skin_image() -> image::DynamicImage {
    use std::io::Cursor;
    image::load(Cursor::new(&include_bytes!("steve.png")[..]),
                            image::PNG).unwrap()
}

fn load_skin_file(ino: &mut INotify, path: &Path) -> (image::DynamicImage, Option<Watch>) {
    let skinfile_watch = match ino.add_watch(path, IN_MODIFY | IN_DELETE_SELF) {
        Ok(wd) => {
            //Yeah...I'm going to go ahead and assume that the
            //path is valid Unicode...
            println!("Watching {}...", path.to_str().unwrap());
            Some(wd)
        },
        Err(e) => {
            //...same here...
            println!("Failed to watch {}!  {}", path.to_str().unwrap(), e.to_string());
            None
        }
    };
    match image::open(path) {
        Ok(img) => (img, skinfile_watch),
        Err(e) => {
            //...and here.
            println!("Failed to load file {} ({}).  Using default skin instead...", path.to_str().unwrap(), e.to_string());
            //Include the watch, if it was created successfully.
            //This should be interesting...
            (load_default_skin_image(), skinfile_watch)
        }
    }
}

fn load_skin(display: &GlutinFacade, ino: &mut INotify, skinfile: &Option<String>, mc17: bool) -> PlayerModel {
    use std::fs;

    let (image, skinfile_watch) = match skinfile {
        &Some(ref filename) =>
        {
            let path = Path::new(&filename);
            //This should be changed to path.exists() once that
            //API is stable.
            match fs::metadata(&path) {
                Ok(_) => load_skin_file(ino, &path),
                Err(e) => {
                    println!("No such file {} ({})", &filename, e.to_string());
                    (load_default_skin_image(), None)
                }
            }
        },
        &None => (load_default_skin_image(), None)
    };

    if mc17 {
        PlayerModel{
            head: ModelPiece::new(display, &steve17::HEAD, PrimitiveType::TrianglesList, None).unwrap(),
            torso: ModelPiece::new(display, &steve17::TORSO, PrimitiveType::TrianglesList, None).unwrap(),

            larm: ModelPiece::new(display, &steve17::LARM, PrimitiveType::TrianglesList, Some(*steve17::LARM_BONE)).unwrap(),
            rarm: ModelPiece::new(display, &steve17::RARM, PrimitiveType::TrianglesList, Some(*steve17::RARM_BONE)).unwrap(),

            lleg: ModelPiece::new(display, &steve17::LLEG, PrimitiveType::TrianglesList, Some(*steve17::LLEG_BONE)).unwrap(),
            rleg: ModelPiece::new(display, &steve17::RLEG, PrimitiveType::TrianglesList, Some(*steve17::RLEG_BONE)).unwrap(),

            texture: SrgbTexture2d::new(display, image).unwrap(),
            texture_watch: skinfile_watch,
        }
    } else {
        PlayerModel{
            head: ModelPiece::new(display, &steve::HEAD, PrimitiveType::TrianglesList, None).unwrap(),
            torso: ModelPiece::new(display, &steve::TORSO, PrimitiveType::TrianglesList, None).unwrap(),

            larm: ModelPiece::new(display, &steve::LARM, PrimitiveType::TrianglesList, Some(*steve::LARM_BONE)).unwrap(),
            rarm: ModelPiece::new(display, &steve::RARM, PrimitiveType::TrianglesList, Some(*steve::RARM_BONE)).unwrap(),

            lleg: ModelPiece::new(display, &steve::LLEG, PrimitiveType::TrianglesList, Some(*steve::LLEG_BONE)).unwrap(),
            rleg: ModelPiece::new(display, &steve::RLEG, PrimitiveType::TrianglesList, Some(*steve::RLEG_BONE)).unwrap(),

            texture: SrgbTexture2d::new(display, image).unwrap(),
            texture_watch: skinfile_watch,
        }
    }
}

enum SkinFileUpdate {
    NoUpdate,
    New(String),
    Modified,
    Deleted,
}

fn get_skin_file_update(ino: &mut INotify) -> SkinFileUpdate {
    use SkinFileUpdate::*;
    let events = ino.available_events().unwrap();
    if events.len() > 1 {
        if events.len() == 2 && events[0].is_ignored() || events[1].is_ignored() {
            //All is well.
        } else {
            println!("More than 1 event ({}) on skin file!  This is highly irregular.", events.len());
        }
    }
    if events.len() == 0 {
        return NoUpdate;
    }
    let event = &events[0];
    if event.is_dir() {
        println!("Directory event?  Ignore that!");
        return NoUpdate;
    }
    if event.is_modify() {
        println!("Modification of {}", &event.name);
        Modified
    } else if event.is_create() {
        New(event.name.clone())
    } else if event.is_delete_self() {
        Deleted
    } else {
        NoUpdate
    }
 }

fn mainloop(display: &GlutinFacade, ino: &mut INotify, skinfile: Option<String>, mc17: bool) {
    use SkinFileUpdate::*;

    let mut player = load_skin(display, ino, &skinfile, mc17);
    let shader_prog = Program::from_source(display, VERT_PROG, FRAG_PROG, None).unwrap();

    let mut t = 0.0f32;
    let mut angle_y = 0.0f32;
    let mut angle_x = 0.0f32;

    let anim_rate = 0.04f32;
    let mut turn_rate_y = 0.0f32;
    let mut turn_rate_x = 0.0f32;

    let mut do_anim = false;

    let mut mouse_state = MouseState{
        left_pressed: false,
        position: None,
    };

    loop {
        let skinfile_update = get_skin_file_update(ino);
        match skinfile_update {
            Modified => {
                println!("Skin file modified.");
                player = load_skin(display, ino, &skinfile, mc17);
            },
            New(path) => {
                player = load_skin(display, ino, &Some(path), mc17);
            },
            Deleted => {
                println!("Skin file deleted.");
                //No need to remove the underlying watch object; inotify
                //takes care of that for us.
                player.texture_watch = None;
                player = load_skin(display, ino, &skinfile, mc17);
            },
            NoUpdate => ()
        }

        if do_anim {
            t += anim_rate;
        }
        angle_y += turn_rate_y;
        angle_x += turn_rate_x;

        for ev in display.poll_events() {
            match ev {
                Event::Closed => return,
                Event::KeyboardInput(state, _, vk_opt) => match handle_input(&mut turn_rate_y, &mut turn_rate_x, &mut do_anim, &mut t, state, &vk_opt) {
                        Some(NextAction::Quit) => return,
                        Some(NextAction::Reload) => player = load_skin(display, ino, &skinfile, mc17),
                        None => ()
                },
                Event::MouseInput(state, button) => handle_mouse_button(button, state, &mut mouse_state),
                Event::MouseMoved((x, y)) => handle_mouse_motion((x, y), &mut mouse_state, &mut angle_y, &mut angle_x),
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

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    use glium::{DisplayBuild, GliumCreationError};
    use glium::glutin::{WindowBuilder, GlRequest, Api, GlProfile};

    let mut ino = INotify::init().unwrap();

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("s", "skin", "set skin file", "SKINFILE");
    opts.optflag("m", "mc17", "use Minecraft 1.7 skin layout");
    opts.optflag("h", "help", "print help (what you're looking at right now)");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            println!("{}", f.to_string());
            print_usage(&program, opts);
            std::process::exit(1);
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let mc17 = matches.opt_present("m");
    let skinfile = matches.opt_str("s");

    let display_option = WindowBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
        .with_gl_profile(GlProfile::Core)
        .with_depth_buffer(24)
        .with_vsync()
        .build_glium();
    match display_option {
        Ok(display) => mainloop(&display, &mut ino, skinfile, mc17),
        Err(creation_error) => match creation_error {
            GliumCreationError::BackendCreationError(_) => println!("Oh, crap!"),
            GliumCreationError::IncompatibleOpenGl(msg) => println!("Incompatible OpenGL: {}", msg)
        }
    }
}
