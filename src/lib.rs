use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Index;

use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

struct Context
{
    window_height: f32,
    window_width: f32,
    snake_pos: Vec<f32>
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

    context.viewport(0, 0, 1280, 800);

    let vertex_shader = compile_shader
    (
        &context,
        WebGl2RenderingContext::VERTEX_SHADER,
        r##"#version 300 es

        in vec2 position;
        in vec2 translation;

        uniform vec2 resolution;

        void main() {
            vec2 translated = position + translation;
            vec2 zeroToOne = translated / resolution;
            vec2 zeroToTwo = zeroToOne * 2.0;
            vec2 clipSpace = zeroToTwo - 1.0;
            gl_Position = vec4(clipSpace, 0, 1);
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
            outColor = vec4(0.1, 0.65, 0.1, 1);
        }
        "##
    )?;

    let program = link_program(&context, &vertex_shader, &fragment_shader)?;
    context.use_program(Some(&program));

    let window_width = canvas.width() as f32;
    let window_height = canvas.height() as f32;

    let resolution_location = context.get_uniform_location(&program, "resolution").unwrap();
    context.uniform2f(Some(&resolution_location), window_width, window_height);

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    unsafe {
        let mut ctx = Context {
            window_width,
            window_height,
            snake_pos: create_box
            (
                window_width - GRID_BOX_WIDTH,
                window_height - GRID_BOX_HEIGHT,
                GRID_BOX_WIDTH,
                GRID_BOX_HEIGHT,
            )
        };

        *g.borrow_mut() = Some(Closure::new(move || {
            let mut vertices_count = ctx.snake_pos.len();

            QUEUED_ANIMATIONS.retain(|a| !a.done());

            for active_key in &KEYS { handle_key_action(&mut ctx, *active_key); }

            let mut renders: Vec<(Vec<f32>, Vec<f32>)> = vec![];
            let mut end_position = ctx.snake_pos.clone();

            for animation in QUEUED_ANIMATIONS.iter() {
                if animation.done() { continue }

                let elapsed              = js_sys::Date::now() - animation.start_time;
                let interpolation_factor = elapsed / animation.duration;
                let interpolation_factor = interpolation_factor as f32;

                for i in (0..vertices_count).step_by(2) {
                    let start_x = animation.start_position[i];
                    let start_y = animation.start_position[i + 1];

                    let dx = animation.end_position[i] - start_x;
                    let dy = animation.end_position[i + 1] - start_y;

                    end_position[i]     = ((start_x + dx * interpolation_factor) / 10.).round() * 10.;
                    end_position[i + 1] = ((start_y + dy * interpolation_factor) / 10.).round() * 10.;
                }
            }

            renders.push((ctx.snake_pos.clone(), end_position.clone()));

            let x = end_position[0];
            let y = end_position[1];

            if x >= ctx.window_width {
                // end_position = create_box(0., end_position[1], GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
                end_position = create_box(0., end_position[1], GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
            }

            if x + GRID_BOX_WIDTH > ctx.window_width {
                let width = (x + GRID_BOX_WIDTH) - ctx.window_width;
                let width = width.min(GRID_BOX_WIDTH);

                let vertices = create_box(0., y, width, GRID_BOX_HEIGHT);
                renders.push((vertices.clone(), vertices.clone()));

                // end_position = create_box(x, y, GRID_BOX_WIDTH - width, GRID_BOX_HEIGHT);
            }

            if x + GRID_BOX_WIDTH <= 0. {
                end_position = create_box(ctx.window_width - GRID_BOX_WIDTH, y, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
            }

            if x <= 0. {
                let hidden_width = 0. - x;
                let vertices = create_box(ctx.window_width - hidden_width, y, hidden_width, GRID_BOX_HEIGHT);
                renders.push((vertices.clone(), vertices.clone()));
            }

            // Above

            if y >= ctx.window_height {
                end_position = create_box(x, 0., GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
            }

            if y + GRID_BOX_HEIGHT >= ctx.window_height {
                let height = y - ctx.window_height;
                let vertices = create_box(x, height, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
                renders.push((vertices.clone(), vertices.clone()));
            }

            // Below

            if y + GRID_BOX_HEIGHT <= 0. {
                end_position = create_box(x, ctx.window_height - GRID_BOX_HEIGHT, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
            }

            if y <= 0. {
                let height = y.abs();
                let vertices = create_box(x, ctx.window_height - height, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
                renders.push((vertices.clone(), vertices.clone()));
            }

            context.clear_color(0.1, 0.2, 0.1, 1.0);
            context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

            for (initial, resulting) in &renders {
                draw_vertices(&context, &program, initial, resulting).expect("Drawing failed");
                render(&context, (initial.len() / 2) as i32);
            }

            ctx.snake_pos = end_position;
            // log(&format!("[{}, {}]", ctx.snake_pos[0], ctx.snake_pos[1]));

            request_animation_frame(f.borrow().as_ref().unwrap());
        }));

        request_animation_frame(g.borrow().as_ref().unwrap());
    }

    Ok(())
}

fn create_box(x: f32, y: f32, width: f32, height: f32) -> Vec<f32>
{
    vec![
        x,         y,
        x + width, y,
        x,         y + height,

        x + width, y + height,
        x,         y + height,
        x + width, y
    ]
}

const ASPECT_RATIO: f32 = 1.6;

const GRID_WIDTH: usize  = 16;
const GRID_HEIGHT: usize = 10;

const GRID_BOX_WIDTH: f32 = 1280. / GRID_WIDTH as f32;
const GRID_BOX_HEIGHT: f32 = 800. / GRID_HEIGHT as f32;

static mut QUEUED_ANIMATIONS: Vec<Animation> = vec![];

struct Animation
{
    key_code: u32,
    start_time: f64,
    duration: f64,
    start_position: Vec<f32>,
    end_position: Vec<f32>
}

impl Animation {
    fn done(&self) -> bool
    {
        (js_sys::Date::now() - self.start_time) >= self.duration
    }
}

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

fn handle_key_action(ctx: &mut Context, key: u32)
{
    const ANIMATION_DURATION: f64 = 200.;
    // const STEP: f32               = 1280. / 64.;
    // const STEP: f32 = 16.;
    const STEP: f32 = GRID_BOX_WIDTH;


    unsafe {
        if !QUEUED_ANIMATIONS.is_empty() { return }

        match key {
            87 /* w */  => {
                let mut end_position = ctx.snake_pos.clone();
                for i in (1..end_position.len()).step_by(2) {
                    end_position[i] += STEP;
                }

                QUEUED_ANIMATIONS.push
                (
                    Animation {
                        key_code: key,
                        start_time: js_sys::Date::now(),
                        duration: ANIMATION_DURATION,
                        start_position: ctx.snake_pos.clone(),
                        end_position,
                    }
                );
            },
            83 /* s */ => {
                let mut end_position = ctx.snake_pos.clone();
                for i in (1..end_position.len()).step_by(2) {
                    end_position[i] -= STEP;
                }

                QUEUED_ANIMATIONS.push
                (
                    Animation {
                        key_code: key,
                        start_time: js_sys::Date::now(),
                        duration: ANIMATION_DURATION,
                        start_position: ctx.snake_pos.clone(),
                        end_position,
                    }
                );
            },
            65 /* a */ => {
                let mut end_position = ctx.snake_pos.clone();
                for i in (0..end_position.len()).step_by(2) {
                    end_position[i] -= STEP;
                }

                QUEUED_ANIMATIONS.push
                (
                    Animation {
                        key_code: key,
                        start_time: js_sys::Date::now(),
                        duration: ANIMATION_DURATION,
                        start_position: ctx.snake_pos.clone(),
                        end_position
                    }
                );
            },
            68 /* d */ => {
                let mut end_position = ctx.snake_pos.clone();
                for i in (0..end_position.len()).step_by(2) {
                    end_position[i] += STEP;
                }

                QUEUED_ANIMATIONS.push
                (
                    Animation {
                        key_code: key,
                        start_time: js_sys::Date::now(),
                        duration: ANIMATION_DURATION,
                        start_position: ctx.snake_pos.clone(),
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
        let code = event.key_code();
        match code {
            87 | 83 | 65 | 68 => {
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
    vertices: &[f32],
    translation: &[f32]
) -> Result<(), String>
{
    // Position

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
    context.vertex_attrib_pointer_with_i32(position_attribute_location as u32, 2, WebGl2RenderingContext::FLOAT, false, 0, 0);
    context.enable_vertex_attrib_array(position_attribute_location as u32);

    context.bind_vertex_array(Some(&vao));

    // Translation

    let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
    context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

    let positions_array_buf = js_sys::Float32Array::new_with_length(translation.len() as u32);
    positions_array_buf.copy_from(translation);

    context.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &positions_array_buf,
        WebGl2RenderingContext::DYNAMIC_DRAW,
    );

    let vao = context.create_vertex_array().ok_or("Failed to create vertex array object")?;
    context.bind_vertex_array(Some(&vao));

    let position_attribute_location = context.get_attrib_location(program, "translation");
    context.vertex_attrib_pointer_with_i32(position_attribute_location as u32, 2, WebGl2RenderingContext::FLOAT, false, 0, 0);
    context.enable_vertex_attrib_array(position_attribute_location as u32);

    context.bind_vertex_array(Some(&vao));

    Ok(())
}

fn render(context: &WebGl2RenderingContext, vert_count: i32)
{
    // context.clear_color(0.0, 0.0, 0.0, 1.0);
    // context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
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
