use std::rc::Rc;
use std::cell::RefCell;
use std::time;
use js_sys::Intl::PluralRules;

use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue>
{
    let window = web_sys::window().unwrap();

    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let context = canvas
        .get_context("webgl2")?
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()?;

    let vertex_shader = compile_shader
    (
        &context,
        WebGl2RenderingContext::VERTEX_SHADER,
        r##"#version 300 es

        in vec4 position;

        void main() {
            gl_Position = position;
        }
        "##,
    )?;

    let fragment_shader = compile_shader
    (
        &context,
        WebGl2RenderingContext::FRAGMENT_SHADER,
        r##"#version 300 es

        precision highp float;
        out vec4 outColor;

        void main() {
            outColor = vec4(1, 1, 1, 1);
        }
        "##
    )?;

    let program = link_program(&context, &vertex_shader, &fragment_shader)?;
    context.use_program(Some(&program));

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    unsafe {
        let vertices_count = VERTICES.len();

        *g.borrow_mut() = Some(Closure::new(move || {
            QUEUED_ANIMATIONS.retain(|a| !a.done());

            for active_key in &KEYS {
                handle_key_action(*active_key);
            }

            for animation in QUEUED_ANIMATIONS.iter() {
                if animation.done() { continue }

                let elapsed              = js_sys::Date::now() - animation.start_time;
                let interpolation_factor = (elapsed / animation.duration).min(1.);
                let time_factor: f32     = interpolation_factor as f32;

                for i in (0..vertices_count).step_by(3) {
                    let dx = animation.end_position[i] - VERTICES[i];
                    let dy = animation.end_position[i + 1] - VERTICES[i + 1];

                    let length = f32::sqrt(dx * dx + dy * dy);
                    let angle_factor = 15.0 * length;

                    VERTICES[i]     += angle_factor * time_factor * dx;
                    VERTICES[i + 1] += angle_factor * time_factor * dy;
                }
            }

            let vert_count = (vertices_count / 3) as i32;
            draw_vertices(&context, &program, &VERTICES).expect("Drawing failed");
            render(&context, vert_count);

            request_animation_frame(f.borrow().as_ref().unwrap());
        }));

        request_animation_frame(g.borrow().as_ref().unwrap());
    }

    Ok(())
}

#[derive(Debug)]
struct Vec2 {
    x: f32,
    y: f32
}

fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2{x, y}
}

static mut QUEUED_ANIMATIONS: Vec<Animation> = vec![];

struct Animation
{
    start_time: f64,
    duration: f64,
    end_position: [f32;18]
}

impl Animation {
    fn done(&self) -> bool
    {
        (js_sys::Date::now() - self.start_time) >= self.duration
    }
}

static mut VERTICES: [f32; 18] = [
   -0.1, -0.1, 0.0,
    0.1, -0.1, 0.0,
   -0.1,  0.1, 0.0,

    0.1,  0.1, 0.0,
   -0.1,  0.1, 0.0,
    0.1, -0.1, 0.0,
];

static mut KEYS: Vec<u32> = vec![];

fn window() -> web_sys::Window
{
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut()>)
{
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn handle_key_action(key: u32)
{
    const ANIMATION_DURATION: f64 = 100.;
    const STEP: f32               = 0.025;

    unsafe {
        match key {
            87 /* w */  => {
                let mut end_position = VERTICES.clone();
                for i in (1..end_position.len()).step_by(3) {
                    end_position[i] += STEP;
                }

                QUEUED_ANIMATIONS.push
                (
                    Animation {
                        start_time: js_sys::Date::now(),
                        duration: ANIMATION_DURATION,
                        end_position,
                    }
                );
            },
            83 /* s */ => {
                let mut end_position = VERTICES.clone();
                for i in (1..end_position.len()).step_by(3) {
                    end_position[i] -= STEP;
                }

                QUEUED_ANIMATIONS.push
                (
                    Animation {
                        start_time: js_sys::Date::now(),
                        duration: ANIMATION_DURATION,
                        end_position,
                    }
                );
            },
            65 /* a */ => {
                let mut end_position = VERTICES.clone();
                for i in (0..end_position.len()).step_by(3) {
                    end_position[i] -= STEP;
                }

                QUEUED_ANIMATIONS.push
                (
                    Animation {
                        start_time: js_sys::Date::now(),
                        duration: ANIMATION_DURATION,
                        end_position
                    }
                );
            },
            68 /* d */ => {
                let mut end_position = VERTICES.clone();
                for i in (0..end_position.len()).step_by(3) {
                    end_position[i] += STEP;
                }

                QUEUED_ANIMATIONS.push
                (
                    Animation {
                        start_time: js_sys::Date::now(),
                        duration: ANIMATION_DURATION,
                        end_position
                    }
                );
            },
            _   => ()
        }
    }
}

#[wasm_bindgen]
pub fn key_down_event(event: web_sys::KeyboardEvent) -> Result<(), JsValue>
{
    unsafe {
        match event.key().as_str() {
            "w" | "s" | "a" | "d" => {
                let code = event.key_code();

                if !KEYS.contains(&code) {
                    KEYS.push(event.key_code())
                }
            },
            _   => ()
        }
    }

    Ok(())
}

#[wasm_bindgen]
pub fn key_up_event(event: web_sys::KeyboardEvent) -> Result<(), JsValue>
{
    unsafe {
        match event.key().as_str() {
            "w" | "s" | "a" | "d" => KEYS.retain(|c| c != &event.key_code()),
            _   => ()
        }
    }

    Ok(())
}

fn draw_vertices(
    context: &WebGl2RenderingContext,
    program: &WebGlProgram,
    vertices: &[f32]
) -> Result<(), String>
{
    let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
    context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

    let positions_array_buf = js_sys::Float32Array::new_with_length(vertices.len() as u32);
    positions_array_buf.copy_from(vertices);

    context.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &positions_array_buf,
        WebGl2RenderingContext::DYNAMIC_DRAW,
    );

    let vao = context.create_vertex_array().ok_or("Failed to create vertex array object")?;
    context.bind_vertex_array(Some(&vao));

    let position_attribute_location = context.get_attrib_location(program, "position");
    context.vertex_attrib_pointer_with_i32(position_attribute_location as u32, 3, WebGl2RenderingContext::FLOAT, false, 0, 0);
    context.enable_vertex_attrib_array(position_attribute_location as u32);

    context.bind_vertex_array(Some(&vao));

    Ok(())
}

fn render(context: &WebGl2RenderingContext, vert_count: i32)
{
    context.clear_color(0.0, 0.0, 0.0, 1.0);
    context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
    context.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, vert_count);
}

pub fn compile_shader(
    context: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str
) -> Result<WebGlShader, String>
{
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    let compiled = context
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false);

    match compiled {
        true  => Ok(shader),
        false => Err(
            context
                .get_shader_info_log(&shader)
                .unwrap_or_else(|| String::from("Uknown error creating shader"))
        )
    }
}

pub fn link_program(
    context: &WebGl2RenderingContext,
    vertex_shader: &WebGlShader,
    fragment_shader: &WebGlShader,
) -> Result<WebGlProgram, String>
{
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create program"))?;

    context.attach_shader(&program, vertex_shader);
    context.attach_shader(&program, fragment_shader);

    context.link_program(&program);

    let program_linked = context
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false);

    match program_linked {
        true  => Ok(program),
        false => Err(
            context.get_program_info_log(&program)
                .unwrap_or_else(|| String::from("Unknown error creating program object"))
        )
    }
}