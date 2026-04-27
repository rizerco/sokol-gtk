use std::sync::atomic::{AtomicUsize, Ordering};

use gdk::GLAPI;
use gdk::glib::Propagation;
use gl::types::GLint;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use rand::Rng;
use sokol::gfx::VertexFormat;
use sokol::gfx::{self as sg};

mod shader;

#[derive(Default)]
struct State {
    bind: sg::Bindings,
    pip: sg::Pipeline,
    swapchain: sg::Swapchain,
    clear_color: sg::Color,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

static POINTER_ADDRESS: AtomicUsize = AtomicUsize::new(0);

fn state_from_pointer<'a>(state_pointer: usize) -> Option<&'a mut State> {
    unsafe {
        let state = state_pointer as *mut State;
        state.as_mut()
    }
}

impl Drop for State {
    fn drop(&mut self) {
        println!("State is dropped");
    }
}

extern "C" fn init() {
    let state = get_state();
    state.bind.vertex_buffers[0] = sg::make_buffer(&sg::BufferDesc {
        #[rustfmt::skip]
        data: sg::value_as_range::<[f32; _]>(&[
             // positions    colors
             0.0,  0.5, 0.5, 1.0, 0.0, 0.0, 1.0,
             0.5, -0.5, 0.5, 0.0, 1.0, 0.0, 1.0,
            -0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 1.0,
        ]),
        ..Default::default()
    });

    // create a shader and pipeline object
    state.pip = sg::make_pipeline(&sg::PipelineDesc {
        shader: sg::make_shader(&shader::triangle_shader_desc(sg::query_backend())),
        layout: {
            let mut l = sg::VertexLayoutState::new();
            l.attrs[shader::ATTR_TRIANGLE_POSITION].format = VertexFormat::Float3;
            l.attrs[shader::ATTR_TRIANGLE_COLOR0].format = VertexFormat::Float4;
            l
        },
        ..Default::default()
    });
}

extern "C" fn frame(area: &gtk::GLArea) {
    let state = get_state();

    let mut framebuffer_id: GLint = 0;
    unsafe {
        epoxy::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut framebuffer_id);
    }

    state.swapchain.width = area.width();
    state.swapchain.height = area.height();
    state.swapchain.gl = sg::GlSwapchain {
        framebuffer: framebuffer_id as u32,
    };

    let mut pass_action = sg::PassAction::new();
    pass_action.colors[0].load_action = sg::LoadAction::Clear;
    pass_action.colors[0].clear_value = state.clear_color;

    sg::begin_pass(&sg::Pass {
        action: pass_action,
        swapchain: state.swapchain,
        ..Default::default()
    });
    sg::apply_pipeline(state.pip);
    sg::apply_bindings(&state.bind);
    sg::draw(0, 3, 1);
    sg::end_pass();
    sg::commit();
}

fn create_window(app: &gtk::Application) {
    let window = ApplicationWindow::new(app);
    window.set_default_size(800, 600);

    let state = State::default();
    let state = Box::new(state);
    let state_pointer = &*state as *const State as usize;
    POINTER_ADDRESS.store(state_pointer, Ordering::SeqCst);
    // Store the state in the app data so it doesn’t get dropped.
    unsafe { app.set_data("state", state) };

    let gl_area = gtk::GLArea::new();
    gl_area.set_vexpand(true);
    gl_area.set_hexpand(true);
    gl_area.set_auto_render(true);
    gl_area.set_allowed_apis(GLAPI::GL);

    gl_area.connect_realize(|area| {
        area.make_current();

        if area.error().is_some() {
            println!("Error creating GLArea context");
            return;
        }

        sg::setup(&sg::Desc {
            environment: sg::Environment {
                defaults: sg::EnvironmentDefaults {
                    color_format: sg::PixelFormat::Rgba8,
                    depth_format: sg::PixelFormat::None,
                    ..Default::default()
                },
                ..Default::default()
            },
            logger: sg::Logger {
                func: Some(sokol::log::slog_func),
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(sg::isvalid());
        init();
    });

    gl_area.connect_render(move |area, _context| {
        if !area.is_realized() {
            return Propagation::Stop;
        }
        frame(area);
        Propagation::Proceed
    });

    let button = gtk::Button::with_label("Click me!");
    button.set_halign(gtk::Align::Start);
    button.set_valign(gtk::Align::Start);
    button.set_margin_start(8);
    button.set_margin_top(8);

    let area_clone = gl_area.clone();
    button.connect_clicked(move |_| {
        randomize_clear_color();
        area_clone.queue_draw();
    });

    let overlay = gtk::Overlay::new();
    overlay.add_overlay(&gl_area);
    overlay.add_overlay(&button);

    window.set_child(Some(&overlay));

    window.present();
}

fn main() {
    let library = unsafe { libloading::os::unix::Library::new("libepoxy.so.0") }.unwrap();
    epoxy::load_with(|name| {
        unsafe { library.get::<_>(name.as_bytes()) }
            .map(|symbol| *symbol)
            .unwrap_or(std::ptr::null())
    });

    let app = Application::new(Some("com.examplstate.SokolGtkApp"), Default::default());

    app.connect_activate(|app| {
        create_window(app);
    });

    app.run();
}

fn randomize_clear_color() {
    let state = get_state();
    let mut rng = rand::rng();
    state.clear_color.r = rng.random_range(0.0..0.2);
    state.clear_color.g = rng.random_range(0.0..0.2);
    state.clear_color.b = rng.random_range(0.0..0.2);
}

fn get_state<'a>() -> &'a mut State {
    let address = POINTER_ADDRESS.load(Ordering::SeqCst);
    state_from_pointer(address).unwrap()
}
