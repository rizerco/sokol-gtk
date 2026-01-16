use std::cell::RefCell;

use gdk::glib::Propagation;
use gl::types::GLint;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use sokol::gfx::VertexFormat;
use sokol::gfx::{self as sg};

mod shader;

#[derive(Default)]
struct State {
    bind: sg::Bindings,
    pip: sg::Pipeline,
    swapchain: sg::Swapchain,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

impl Drop for State {
    fn drop(&mut self) {
        println!("State is dropped");
    }
}

extern "C" fn init() {
    // create vertex buffer with triangle vertices
    STATE.with(|state| {
        state.borrow_mut().bind.vertex_buffers[0] = sg::make_buffer(&sg::BufferDesc {
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
        state.borrow_mut().pip = sg::make_pipeline(&sg::PipelineDesc {
            shader: sg::make_shader(&shader::triangle_shader_desc(sg::query_backend())),
            layout: {
                let mut l = sg::VertexLayoutState::new();
                l.attrs[shader::ATTR_TRIANGLE_POSITION].format = VertexFormat::Float3;
                l.attrs[shader::ATTR_TRIANGLE_COLOR0].format = VertexFormat::Float4;
                l
            },
            ..Default::default()
        });
    });
}

extern "C" fn frame(area: &gtk::GLArea) {
    // let state = unsafe { &mut *(user_data as *mut State) };

    let mut framebuffer_id: GLint = 0;
    unsafe {
        epoxy::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut framebuffer_id);
    }

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.swapchain.width = area.allocated_width();
        state.swapchain.height = area.allocated_height();
        state.swapchain.gl = sg::GlSwapchain {
            framebuffer: framebuffer_id as u32,
        };

        let mut pass_action = sg::PassAction::new();
        pass_action.colors[0].load_action = sg::LoadAction::Clear;
        pass_action.colors[0].clear_value = sg::Color {
            r: 0.0,
            g: 0.0,
            b: 0.2,
            a: 1.0,
        };

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
    });
}

fn create_window(app: &Application) {
    let window = ApplicationWindow::new(app);
    window.set_default_size(800, 600);

    let gl_area = gtk::GLArea::new();
    gl_area.set_vexpand(true);
    gl_area.set_hexpand(true);
    gl_area.set_auto_render(true);

    gl_area.connect_realize(|area| {
        area.make_current();

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

    let overlay = gtk::Overlay::new();
    overlay.add_overlay(&gl_area);
    overlay.add_overlay(&button);

    window.add(&overlay);

    window.show_all();
}

fn main() {
    {
        #[cfg(target_os = "macos")]
        let library = unsafe { libloading::os::unix::Library::new("libepoxy.0.dylib") }.unwrap();
        #[cfg(all(unix, not(target_os = "macos")))]
        let library = unsafe { libloading::os::unix::Library::new("libepoxy.so.0") }.unwrap();
        #[cfg(windows)]
        let library = libloading::os::windows::Library::open_already_loaded("libepoxy-0.dll")
            .or_else(|_| libloading::os::windows::Library::open_already_loaded("epoxy-0.dll"))
            .unwrap();

        epoxy::load_with(|name| {
            unsafe { library.get::<_>(name.as_bytes()) }
                .map(|symbol| *symbol)
                .unwrap_or(std::ptr::null())
        });
    }

    let app = Application::new(Some("com.example.SokolGtkApp"), Default::default());

    app.connect_activate(|app| {
        create_window(app);
    });

    app.run();
}
