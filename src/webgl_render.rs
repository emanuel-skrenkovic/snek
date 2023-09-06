use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext, WebGlProgram, WebGlShader};
use crate::render::Renderer;

struct WasmRenderer
{
    context: WebGl2RenderingContext,
    program: WebGlProgram,
    canvas: HtmlCanvasElement
}

impl Renderer for WasmRenderer
{
    fn draw_vertices(&self, vertices: &[f32], colours: &[f32])
    {
        self.context.clear_color(0.1, 0.2, 0.1, 1.0);
        self.context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

        let vertices_count = vertices.len() / 2;
        let vertices_count= vertices_count as i32;

        draw_vertices(&self.context, &self.program, vertices, colours)
            .expect("Drawing failed");

        self.context.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, vertices_count);
    }

    fn window_width(&self) -> f32
    {
        self.canvas.width() as f32
    }

    fn window_height(&self) -> f32
    {
        self.canvas.height() as f32
    }
}

// #[cfg(wasm)]
pub fn new() -> Box<dyn Renderer>
{
    let window = web_sys::window().unwrap();

    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: HtmlCanvasElement = canvas.dyn_into::<HtmlCanvasElement>().unwrap();

    let context = canvas
        .get_context("webgl2")
        .unwrap()
        .unwrap()
        // unwrap()
        // unwrap()
        // unwrap()
        .dyn_into::<WebGl2RenderingContext>()
        .unwrap();

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
        ).unwrap();

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
    ).unwrap();

    let program = link_program(&context, &vertex_shader, &fragment_shader).unwrap();
    context.use_program(Some(&program));

    let window_width = canvas.width() as f32;
    let window_height = canvas.height() as f32;

    let resolution_location = context.get_uniform_location(&program, "resolution").unwrap();
    context.uniform2f(Some(&resolution_location), window_width, window_height);

    let renderer = WasmRenderer { context, program, canvas };
    return Box::new(renderer)
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