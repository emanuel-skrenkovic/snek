use std::rc::Rc;
use std::cell::RefCell;

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
    snake: Vec<f32>
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

    let mut initial   = Vec::with_capacity(2000);
    let mut resulting = Vec::with_capacity(2000);

    unsafe {
        let starting_pos = create_box
        (
            ((window_width / 2.) / GRID_BOX_WIDTH).round() * GRID_BOX_WIDTH,
            ((window_height / 2.) / GRID_BOX_HEIGHT).round() * GRID_BOX_HEIGHT,
            GRID_BOX_WIDTH,
            GRID_BOX_HEIGHT,
        );

        let mut ctx = Context {
            window_width,
            window_height,
            snake: starting_pos
        };

        for i in 0..SNAKE_STARTING_LEN {
            let mut part = create_box
            (
                (((window_width / 2.) / GRID_BOX_WIDTH).round() *  GRID_BOX_WIDTH) - (i as f32 * GRID_BOX_WIDTH),
                ((window_height / 2.) / GRID_BOX_HEIGHT).round() * GRID_BOX_HEIGHT,
                GRID_BOX_WIDTH,
                GRID_BOX_HEIGHT,
            );

            ctx.snake.append(&mut part);
        }

        *g.borrow_mut() = Some(Closure::new(move || {
            if PAUSED {
                request_animation_frame(f.borrow().as_ref().unwrap());
                return
            }

            update_frame(&mut ctx, &mut initial, &mut resulting);

            context.clear_color(0.1, 0.2, 0.1, 1.0);
            context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

            let vertices_count = (initial.len() / 2) as i32;
            draw_vertices(&context, &program, initial.drain(..).as_slice(), resulting.drain(..).as_slice())
                .expect("Drawing failed");
            context.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, vertices_count);

            request_animation_frame(f.borrow().as_ref().unwrap());
        }));

        request_animation_frame(g.borrow().as_ref().unwrap());
    }

    Ok(())
}

unsafe fn update_frame
(
    ctx: &mut Context,
    initial: &mut Vec<f32>,
    resulting: &mut Vec<f32>
)
{
    QUEUED_ANIMATIONS.retain(|a| !a.done());

    for active_key in &KEYS {
        handle_key_action(ctx, *active_key);
    }

    let mut end_position = ctx.snake.clone();

    for animation in QUEUED_ANIMATIONS.iter() {
        if animation.done() { continue }

        let interpolation_factor = (animation.elapsed() / animation.duration) as f32;

        for j in (0..ctx.snake.len()).step_by(12) {
            for i in (0..12).step_by(2) {
                let start_x = animation.start_position[i + j];
                let start_y = animation.start_position[i + 1 + j];

                let dx = animation.end_position[i + j] - start_x;
                let dy = animation.end_position[i + 1 + j] - start_y;

                end_position[i + j]     = ((start_x + dx * interpolation_factor) / 10.).round() * 10.;
                end_position[i + 1 + j] = ((start_y + dy * interpolation_factor) / 10.).round() * 10.;
            }
        }

        // // Calculate the difference once per animation
        // // as it is the same for every point.
        // let dx = animation.end_position[0] - animation.start_position[0];
        // let dy = animation.end_position[1] - animation.start_position[1];
        //
        // let interpolation_factor = (animation.elapsed() / animation.duration) as f32;
        //
        // for i in (0..ctx.snake.len()).step_by(2) {
        //     end_position[i]     = ((animation.start_position[i] + dx * interpolation_factor) / 10.).round() * 10.;
        //     end_position[i + 1] = ((animation.start_position[i + 1] + dy * interpolation_factor) / 10.).round() * 10.;
        // }
    }

    initial.append(&mut ctx.snake.clone());
    resulting.append(&mut end_position.clone());

    let x = end_position[0];
    let y = end_position[1];

    if x >= ctx.window_width {
        end_position = create_box(0., y, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
    }

    if x + GRID_BOX_WIDTH > ctx.window_width {
        let width = (x + GRID_BOX_WIDTH) - ctx.window_width;
        let width = width.min(GRID_BOX_WIDTH);

        let mut vertices = create_box(0., y, width, GRID_BOX_HEIGHT);
        initial.append(&mut vertices.clone());
        resulting.append(&mut vertices);
    }

    if x + GRID_BOX_WIDTH <= 0. {
        end_position = create_box(ctx.window_width - GRID_BOX_WIDTH, y, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
    }

    if x <= 0. {
        let hidden_width = 0. - x;
        let mut vertices = create_box(ctx.window_width - hidden_width, y, hidden_width, GRID_BOX_HEIGHT);
        initial.append(&mut vertices.clone());
        resulting.append(&mut vertices);
    }

    // Above

    if y >= ctx.window_height {
        end_position = create_box(x, 0., GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
    }

    if y + GRID_BOX_HEIGHT >= ctx.window_height {
        let height = y - ctx.window_height;
        let mut vertices = create_box(x, height, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
        initial.append(&mut vertices.clone());
        resulting.append(&mut vertices);
    }

    // Below

    if y + GRID_BOX_HEIGHT <= 0. {
        end_position = create_box(x, ctx.window_height - GRID_BOX_HEIGHT, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
    }

    if y <= 0. {
        let height = y.abs();
        let mut vertices = create_box(x, ctx.window_height - height, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
        initial.append(&mut vertices.clone());
        resulting.append(&mut vertices);
    }



    ctx.snake = end_position;
}

fn pause_animation(animation: &mut Animation)
{
    if animation.is_paused { return }
    if animation.pause_start_time == 0. {
        animation.pause_start_time = now();
    }
    animation.is_paused = true;
}

fn unpause_animation(animation: &mut Animation)
{
    if !animation.is_paused { return }
    animation.pause_end_time = now();
    animation.is_paused = false;
}

fn now() -> f64
{
    js_sys::Date::now()
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

const GRID_WIDTH: usize  = 16;
const GRID_HEIGHT: usize = 10;

const GRID_BOX_WIDTH: f32 = 1280. / GRID_WIDTH as f32;
const GRID_BOX_HEIGHT: f32 = 800. / GRID_HEIGHT as f32;

const ANIMATION_DURATION: f64 = 200.;
const STEP: f32 = GRID_BOX_WIDTH;

const SNAKE_STARTING_LEN: usize = 5;

static mut QUEUED_ANIMATIONS: Vec<Animation> = vec![];
static mut PAUSED: bool = false;

#[derive(Debug)]
struct Animation
{
    start_time: f64,
    duration: f64,
    start_position: Vec<f32>,
    end_position: Vec<f32>,

    is_paused: bool,

    pause_start_time: f64,
    pause_end_time: f64
}

impl Animation
{
    fn done(&self) -> bool
    {
        self.elapsed() >= self.duration
    }

    fn elapsed(&self) -> f64
    {
        let pause_duration = self.pause_end_time - self.pause_start_time;
        now() - pause_duration - self.start_time
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

unsafe fn handle_key_action(ctx: &mut Context, key: u32)
{
    if !QUEUED_ANIMATIONS.is_empty() { return }

    let resulting_position: Option<Vec<f32>> = match key {
        87 /* w */  => {
            let mut end_position = ctx.snake[0..12].to_vec();
            for i in (1..end_position.len()).step_by(2) {
                end_position[i] += STEP;
            }
            Some(move_snake(&ctx.snake, &end_position))
        },
        83 /* s */ => {
            let mut end_position = ctx.snake.clone();
            for i in (1..end_position.len()).step_by(2) {
                end_position[i] -= STEP;
            }
            Some(move_snake(&ctx.snake, &end_position))
        },
        65 /* a */ => {
            let mut end_position = ctx.snake.clone();
            for i in (0..end_position.len()).step_by(2) {
                end_position[i] -= STEP;
            }
            Some(move_snake(&ctx.snake, &end_position))
        },
        68 /* d */ => {
            let mut end_position = ctx.snake.clone();
            for i in (0..end_position.len()).step_by(2) {
                end_position[i] += STEP;
            }
            Some(move_snake(&ctx.snake, &end_position))
        },
        _   => None
    };

    if let Some(resulting_position) = resulting_position {
        QUEUED_ANIMATIONS.push
        (
            Animation {
                start_time: now(),
                duration: ANIMATION_DURATION,
                start_position: ctx.snake.clone(),
                end_position: resulting_position,
                is_paused: false,
                pause_start_time: 0.,
                pause_end_time: 0.
            }
        );
    }
}

#[inline(always)]
fn move_snake(snake: &[f32], head_movement: &[f32]) -> Vec<f32>
{
    let mut resulting_position = vec![0.; snake.len()];
    let mut end_position       = head_movement.to_vec();

    for part in (0..snake.len()).step_by(12) {
        for i in 0..12 {
            resulting_position[part + i] = end_position[i];
        }
        end_position = snake[part..part + 12].to_vec();
    }

    resulting_position
}

/// Registers the pressed key as an event if
/// it is a part of the logic.
/// Stores the pressed key code in global state
/// if it is not already present.
#[wasm_bindgen]
pub unsafe fn key_down_event(event: web_sys::KeyboardEvent)
{
    let code = event.key_code();
    match code {
        87 | 83 | 65 | 68 => { // wasd
            if !KEYS.contains(&code) {
                KEYS.push(event.key_code())
            }
        },
        _   => ()
    }
}

/// Stores the event into the global state that holds all
/// queued events.
#[wasm_bindgen]
pub unsafe fn key_up_event(event: web_sys::KeyboardEvent)
{
    let code = event.key_code();
    match code {
        87 | 83 | 65 | 68 => KEYS.retain(|c| c != &code),
        _ => ()
    }
}

/// Stores the event into the global state that holds all
/// queued events. This is used for the 'keypress' dom event.
#[wasm_bindgen]
pub unsafe fn key_press_event(event: web_sys::KeyboardEvent)
{
    let code = event.key_code();
    if code == 32 {
        let previously_paused = PAUSED;
        PAUSED = !PAUSED;
        match previously_paused {
            true => {
                for animation in QUEUED_ANIMATIONS.iter_mut() {
                    unpause_animation(animation);
                }
            }
            false => {
                for animation in QUEUED_ANIMATIONS.iter_mut() {
                    pause_animation(animation);
                }
            }
        }
    }
}

fn draw_vertices(
    context: &WebGl2RenderingContext,
    program: &WebGlProgram,
    vertices: &[f32],
    translation: &[f32]
) -> Result<(), String>
{
    // Position

    {
        let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

        let positions_array_buf = js_sys::Float32Array::new_with_length(vertices.len() as u32);
        positions_array_buf.copy_from(vertices);

        context.buffer_data_with_array_buffer_view
        (
            WebGl2RenderingContext::ARRAY_BUFFER,
            &positions_array_buf,
            WebGl2RenderingContext::DYNAMIC_DRAW,
        );

        let vao = context.create_vertex_array().ok_or("Failed to create vertex array object")?;
        context.bind_vertex_array(Some(&vao));

        let position_attribute_location = context.get_attrib_location(program, "position") as u32;
        context.vertex_attrib_pointer_with_i32(position_attribute_location, 2, WebGl2RenderingContext::FLOAT, false, 0, 0);
        context.enable_vertex_attrib_array(position_attribute_location);

        context.bind_vertex_array(Some(&vao));
    }

    // Translation

    {
        let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

        let positions_array_buf = js_sys::Float32Array::new_with_length(translation.len() as u32);
        positions_array_buf.copy_from(translation);

        context.buffer_data_with_array_buffer_view
        (
            WebGl2RenderingContext::ARRAY_BUFFER,
            &positions_array_buf,
            WebGl2RenderingContext::DYNAMIC_DRAW,
        );

        let vao = context.create_vertex_array().ok_or("Failed to create vertex array object")?;
        context.bind_vertex_array(Some(&vao));

        let position_attribute_location = context.get_attrib_location(program, "translation") as u32;
        context.vertex_attrib_pointer_with_i32(position_attribute_location, 2, WebGl2RenderingContext::FLOAT, false, 0, 0);
        context.enable_vertex_attrib_array(position_attribute_location);

        context.bind_vertex_array(Some(&vao));
    }

    Ok(())
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
