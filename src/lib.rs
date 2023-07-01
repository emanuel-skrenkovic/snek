use std::rc::Rc;
use std::cell::RefCell;

use js_sys::Math::random;
use wasm_bindgen::prelude::*;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader};

const GRID_WIDTH: usize  = 16;
const GRID_HEIGHT: usize = 10;

const GRID_BOX_WIDTH: f32 = 1280. / GRID_WIDTH as f32;
const GRID_BOX_HEIGHT: f32 = 800. / GRID_HEIGHT as f32;

const ANIMATION_DURATION: f64 = 200.;
const STEP: f32 = GRID_BOX_WIDTH;

const SNAKE_STARTING_LEN: usize = 4;

const SNAKE_COLOUR: [f32; 3] = [0.1, 0.65, 0.1];
const APPLE_COLOUR: [f32; 3] = [0.65, 0.1, 0.1];

static mut QUEUED_ANIMATIONS: Vec<Animation> = vec![];
static mut PAUSED: bool = true;
static mut GAME_OVER: bool = false;

static mut CTX: Context = Context {
    window_height: 0.0,
    window_width: 0.0,
    snake: vec![],
    apple: None,
    direction: Direction::Left,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = window)]
    fn game_over(score: usize);

    #[wasm_bindgen(js_namespace = window)]
    fn scored(score: usize);

    #[wasm_bindgen(js_namespace = window)]
    fn clear_screen();

    #[wasm_bindgen(js_namespace = window)]
    fn pause();
}

struct Context
{
    window_height: f32,
    window_width: f32,
    snake: Vec<f32>,
    apple: Option<(f32, f32)>,
    direction: Direction
}

#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Up = 1,
    Down,
    Left,
    Right
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

        uniform vec2 resolution;

        in vec2 position;

        in vec3 vertexColour;
        out vec3 fragmentColour;

        void main() {
            fragmentColour = vertexColour;

            vec2 zeroToOne = position / resolution;
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

        in vec3 fragmentColour;
        out vec4 outColour;

        void main() {
            outColour = vec4(fragmentColour, 1.0);
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

    let mut resulting_position = Vec::with_capacity(2000);
    let mut colours            = Vec::with_capacity(4000);

    unsafe {
        CTX = Context {
            window_width,
            window_height,
            snake: vec![],
            apple: None,
            direction: Direction::Left
        };

        initiate_game(window_width, window_height);

        *g.borrow_mut() = Some(Closure::new(move || {
            if PAUSED {
                request_animation_frame(f.borrow().as_ref().unwrap());
                return
            }

            if collisions(&CTX) {
                GAME_OVER = true;
                game_over(CTX.snake.len() / 12 - SNAKE_STARTING_LEN);
                initiate_game(window_width, window_height);
            }

            if did_the_snek_eat_the_apple(&CTX) {
                let snake_len = CTX.snake.len();
                let mut new_tail = create_box
                (
                    CTX.snake[snake_len - 12], // x coord of the last block
                    CTX.snake[snake_len - 11], // y coord of the last block
                    GRID_BOX_WIDTH,
                    GRID_BOX_HEIGHT
                );

                CTX.snake.append(&mut new_tail);
                CTX.apple = None;
                scored(CTX.snake.len() / 12 - SNAKE_STARTING_LEN);
            }

            QUEUED_ANIMATIONS.retain(|a| {
                let done = a.done();

                // Finish off the animation movement if it has ended.
                // This is to avoid misalignment of the snake end position
                // if the frame rate does not match the animation end time.
                if done {
                    CTX.snake[0..12].copy_from_slice(&a.end_position[0..12]);
                    block_exceeds_screen_edge(&CTX, &mut CTX.snake[0..12], &mut resulting_position);
                }

                !done
            });

            for active_key in &KEYS {
                handle_key_action(&mut CTX, &mut QUEUED_ANIMATIONS, *active_key);
            }

            snake_movement(&mut CTX, &QUEUED_ANIMATIONS, &mut resulting_position);
            spawn_apple(&mut CTX);

            colours.append(&mut SNAKE_COLOUR.repeat(resulting_position.len() / 2));

            if let Some(apple) = CTX.apple {
                let mut apple_vertices = create_box(apple.0, apple.1, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);

                colours.append(&mut APPLE_COLOUR.repeat(apple_vertices.len() / 2));
                resulting_position.append(&mut apple_vertices);
            }

            let vertices_count = resulting_position.len() / 2;
            let vertices_count= vertices_count as i32;

            context.clear_color(0.1, 0.2, 0.1, 1.0);
            context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

            draw_vertices
            (
                &context,
                &program,
                resulting_position.drain(..).as_slice(),
                colours.drain(..).as_slice()
            ).expect("Drawing failed");

            context.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, vertices_count);

            request_animation_frame(f.borrow().as_ref().unwrap());
        }));

        request_animation_frame(g.borrow().as_ref().unwrap());
    }

    Ok(())
}

#[inline(always)]
fn spawn_apple(ctx: &mut Context)
{
    if ctx.apple.is_some() { return }

    let vertical_blocks   = GRID_WIDTH / GRID_BOX_WIDTH as usize;
    let horizontal_blocks = GRID_HEIGHT / GRID_BOX_HEIGHT as usize;

    let mut unoccupied: Vec<(f32, f32)> = Vec::with_capacity(vertical_blocks * horizontal_blocks);

    for i in (0..GRID_WIDTH * GRID_BOX_WIDTH as usize).step_by(GRID_BOX_WIDTH as usize) {
        for j in (0..GRID_HEIGHT * GRID_BOX_HEIGHT as usize).step_by(GRID_BOX_HEIGHT as usize) {
            let mut occupied = false;

            for k in (0..ctx.snake.len()).step_by(2) {
                let snake = create_box(ctx.snake[k], ctx.snake[k + 1], GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
                let block = create_box(i as f32, j as f32, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
                if !box_collision(&snake, &block) { continue }

                occupied = true;
                break;
            }

            if !occupied { unoccupied.push((i as f32, j as f32)); }
        }
    }

    let seed     = random() * 2000.;
    let seed     = seed as usize;
    let position = seed % unoccupied.len();
    ctx.apple    = Some((unoccupied[position].0, unoccupied[position].1));
}

#[inline(always)]
fn snake_movement(ctx: &mut Context, animations: &[Animation], resulting_position: &mut Vec<f32>)
{
    let mut end_position = ctx.snake.clone();

    for animation in animations {
        if animation.done() { continue }

        let interpolation_factor = (animation.elapsed() / animation.duration) as f32;

        // Lerp the head only. (TODO: last block of the tail as well.)
        for i in 0..12 {
            let delta = animation.end_position[i] - animation.start_position[i];
            end_position[i] = animation.start_position[i] + delta * interpolation_factor;
        }

        for i in 12..animation.start_position.len() {
            // Fills the background of the snake with snake body tiles.
            // This is so "turns" are smoother - they are filled with a snake tile
            // underneath so the corners aren't "smoothed" while turning.
            // Head is excluded so the head movement animation remains smooth.
            // Otherwise the "below" tile would just appear at the end position.
            resulting_position.push(animation.end_position[i]);
            end_position[i] = animation.end_position[i];
        }
    }

    resulting_position.append(&mut ctx.snake.clone());

    for i in (0..end_position.len()).step_by(12) {
        block_exceeds_screen_edge(ctx, &mut end_position[i..i + 12], resulting_position);
    }

    ctx.snake = end_position;
}

unsafe fn initiate_game(window_width: f32, window_height: f32)
{
    PAUSED = true;
    GAME_OVER = false;

    QUEUED_ANIMATIONS.clear();

    CTX.snake = vec![];

    // Start off by going left.
    KEYS.clear();
    KEYS.push(Direction::Left);
    CTX.direction = Direction::Left;

    for i in 0..SNAKE_STARTING_LEN {
        let mut part = create_box
        (
            (((window_width / 2.) / GRID_BOX_WIDTH).round() *  GRID_BOX_WIDTH) + (i as f32 * GRID_BOX_WIDTH),
            ((window_height / 2.) / GRID_BOX_HEIGHT).round() * GRID_BOX_HEIGHT,
            GRID_BOX_WIDTH,
            GRID_BOX_HEIGHT,
        );

        CTX.snake.append(&mut part);
    }
}

#[inline(always)]
fn block_exceeds_screen_edge
(
    ctx: &Context,
    block: &mut [f32],
    resulting_position: &mut Vec<f32>
)
{
    let x = block[0];
    let y = block[1];

    // Right
    if x >= ctx.window_width {
        block.copy_from_slice(&create_box(0., y, GRID_BOX_WIDTH, GRID_BOX_HEIGHT));
    }

    if x + GRID_BOX_WIDTH > ctx.window_width {
        let width = (x + GRID_BOX_WIDTH) - ctx.window_width;
        let width = width.min(GRID_BOX_WIDTH);

        let mut vertices = create_box(0., y, width, GRID_BOX_HEIGHT);
        resulting_position.append(&mut vertices);
    }

    // Left
    if x + GRID_BOX_WIDTH <= 0. {
        block.copy_from_slice
        (
            &create_box(ctx.window_width - GRID_BOX_WIDTH, y, GRID_BOX_WIDTH, GRID_BOX_HEIGHT)
        );
    }

    if x <= 0. {
        let hidden_width = 0. - x;
        let mut vertices = create_box(ctx.window_width - hidden_width, y, hidden_width, GRID_BOX_HEIGHT);
        resulting_position.append(&mut vertices);
    }

    // Up
    if y >= ctx.window_height {
        block.copy_from_slice(&create_box(x, 0., GRID_BOX_WIDTH, GRID_BOX_HEIGHT));
    }

    if y + GRID_BOX_HEIGHT >= ctx.window_height {
        let height = y - ctx.window_height;
        let mut vertices = create_box(x, height, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
        resulting_position.append(&mut vertices);
    }

    // Down
    if y + GRID_BOX_HEIGHT <= 0. {
        block.copy_from_slice
        (
            &create_box(x, ctx.window_height - GRID_BOX_HEIGHT, GRID_BOX_WIDTH, GRID_BOX_HEIGHT)
        );
    }

    if y <= 0. {
        let height = y.abs();
        let mut vertices = create_box(x, ctx.window_height - height, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
        resulting_position.append(&mut vertices);
    }
}

// AABB vs AABB
// https://developer.mozilla.org/en-US/docs/Games/Techniques/3D_collision_detection#aabb_vs._aabb
#[inline(always)]
fn box_collision(one: &[f32], two: &[f32]) -> bool
{
    // a.minX <= b.maxX &&
    // a.maxX >= b.minX &&
    // a.minY <= b.maxY &&
    // a.maxY >= b.minY &&

    let collision_x = one[0] < two[0] + GRID_BOX_WIDTH - 15. && one[0] + GRID_BOX_WIDTH - 15. > two[0];
    let collision_y = one[1] < two[1] + GRID_BOX_HEIGHT - 15. && one[1] + GRID_BOX_HEIGHT - 15. > two[1];
    collision_x && collision_y
}

#[inline(always)]
fn collisions(ctx: &Context) -> bool
{
    let head = &ctx.snake[0..12];
    for i in (36..ctx.snake.len()).step_by(12) {
        if box_collision(head, &ctx.snake[i..i + 12]) { return true }
    }

    false
}

// Head <-> apple collision
#[inline(always)]
fn did_the_snek_eat_the_apple(ctx: &Context) -> bool
{
    let Some(apple) = ctx.apple else { return false };
    let apple_box = create_box(apple.0, apple.1, GRID_BOX_WIDTH, GRID_BOX_HEIGHT);
    box_collision(&ctx.snake[0..12], &apple_box)
}

#[allow(dead_code)]
fn format_coordinates(coordinates: &[f32]) -> String {
    let mut formatted_string = String::new();

    for i in 0..coordinates.len() / 2 {
        let x = coordinates[i * 2];
        let y = coordinates[i * 2 + 1];

        let pair = format!("({:.1}, {:.1})", x, y);

        if i > 0 {
            formatted_string.push_str(", ");
        }

        formatted_string.push_str(&pair);
    }

    formatted_string
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

static mut KEYS: Vec<Direction> = vec![];

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

fn handle_key_action(ctx: &mut Context, animations: &mut Vec<Animation>, key: Direction)
{
    if !animations.is_empty() { return }

    let resulting_position: Option<Vec<f32>> = match key {
        // 87 | 119 | 38 /* w or up arrow */  => {
        Direction::Up => {
            if ctx.direction == Direction::Down { return }
            let mut end_position = ctx.snake[0..12].to_vec();
            for i in (1..end_position.len()).step_by(2) {
                end_position[i] = ((end_position[i]) / 10.).round() * 10. + STEP;
            }
            Some(move_snake(&ctx.snake, &end_position))
        },
        // 83 | 115 | 40 /* s or down arrow */ => {
        Direction::Down => {
            if ctx.direction == Direction::Up { return }
            let mut end_position = ctx.snake.clone();
            for i in (1..end_position.len()).step_by(2) {
                end_position[i] = ((end_position[i]) / 10.).round() * 10. - STEP;
            }
            Some(move_snake(&ctx.snake, &end_position))
        },
        // 65 | 97 | 37 /* a or left arrow */ => {
        Direction::Left => {
            if ctx.direction == Direction::Right { return }
            let mut end_position = ctx.snake.clone();
            for i in (0..end_position.len()).step_by(2) {
                end_position[i] = ((end_position[i]) / 10.).round() * 10. - STEP;
            }
            Some(move_snake(&ctx.snake, &end_position))
        },
        // 68 | 100 | 39 /* d or right arrow */ => {
        Direction::Right => {
            if ctx.direction == Direction::Left { return }
            let mut end_position = ctx.snake.clone();
            for i in (0..end_position.len()).step_by(2) {
                end_position[i] = ((end_position[i]) / 10.).round() * 10. + STEP;
            }
            Some(move_snake(&ctx.snake, &end_position))
        },
    };

    if let Some(resulting_position) = resulting_position {
        ctx.direction = key;
        animations.push
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
        resulting_position[part..part + 12].copy_from_slice(&end_position[..12]);
        end_position = snake[part..part + 12].to_vec();
    }

    resulting_position
}



/// Stores the event into the global state that holds all
/// queued events. This is used for the 'keypress' dom event.
#[wasm_bindgen]
pub unsafe fn key_press_event(event: web_sys::KeyboardEvent)
{
    match event.key_code() {
        // w
        119 | 87 | 38 => {
            if CTX.direction == Direction::Down { return }
            if !KEYS.contains(&Direction::Up) && !KEYS.contains(&Direction::Down) {
                KEYS.clear();
                KEYS.push(Direction::Up)
            }
        }

        // s
        115 | 83 | 40 => {
            if CTX.direction == Direction::Up { return }
            if !KEYS.contains(&Direction::Down) && !KEYS.contains(&Direction::Up) {
                KEYS.clear();
                KEYS.push(Direction::Down)
            }
        }

        // d
        100 | 68 | 39 => {
            if CTX.direction == Direction::Left { return }
            if !KEYS.contains(&Direction::Right) && !KEYS.contains(&Direction::Left) {
                KEYS.clear();
                KEYS.push(Direction::Right)
            }
        }

        // a
        97 | 65 | 37 => {
            if CTX.direction == Direction::Right { return }
            if !KEYS.contains(&Direction::Left) && !KEYS.contains(&Direction::Right) {
                KEYS.clear();
                KEYS.push(Direction::Left)
            }
        }

        32 => {
            let previously_paused = PAUSED;
            PAUSED = !PAUSED;
            match previously_paused {
                true => {
                    clear_screen();
                    for animation in QUEUED_ANIMATIONS.iter_mut() {
                        unpause_animation(animation);
                    }
                }
                false => {
                    pause();
                    for animation in QUEUED_ANIMATIONS.iter_mut() {
                        pause_animation(animation);
                    }
                }
            }
        }
        _ => ()
    }
}

fn draw_vertices
(
    context: &WebGl2RenderingContext,
    program: &WebGlProgram,
    vertices: &[f32],
    colours: &[f32]
) -> Result<(), String>
{
    let vao = context.create_vertex_array().ok_or("Failed to create vertex array object")?;
    context.bind_vertex_array(Some(&vao));

    // Position

    {
        let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

        let array_buf = js_sys::Float32Array::new_with_length(vertices.len() as u32);
        array_buf.copy_from(vertices);

        context.buffer_data_with_array_buffer_view
        (
            WebGl2RenderingContext::ARRAY_BUFFER,
            &array_buf,
            WebGl2RenderingContext::DYNAMIC_DRAW,
        );

        let attrib_location = context.get_attrib_location(program, "position") as u32;
        context.vertex_attrib_pointer_with_i32
        (
            attrib_location, 2, WebGl2RenderingContext::FLOAT, false, 0, 0
        );
        context.enable_vertex_attrib_array(attrib_location);
    }

    // Colour

    {
        let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

        let array_buf = js_sys::Float32Array::new_with_length(colours.len() as u32);
        array_buf.copy_from(colours);

        context.buffer_data_with_array_buffer_view
        (
            WebGl2RenderingContext::ARRAY_BUFFER,
            &array_buf,
            WebGl2RenderingContext::DYNAMIC_DRAW,
        );

        let attrib_location = context.get_attrib_location(program, "vertexColour") as u32;
        context.vertex_attrib_pointer_with_i32
        (
            attrib_location, 3, WebGl2RenderingContext::FLOAT, false, 0, 0
        );
        context.enable_vertex_attrib_array(attrib_location);
    }

    Ok(())
}

pub fn compile_shader
(
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
                .unwrap_or_else(|| String::from("Unknown error creating shader"))
        )
    }
}

pub fn link_program
(
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
