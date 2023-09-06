pub trait Renderer
{
    fn draw_vertices(&self, vertices: &[f32], colours: &[f32]);
    fn window_width(&self) -> f32;
    fn window_height(&self) -> f32;
}