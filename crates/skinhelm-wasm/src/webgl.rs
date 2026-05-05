use js_sys::Float32Array;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlTexture, WebGlUniformLocation,
};

use crate::animation::WalkPose;
use crate::error::ViewerError;
use crate::math::Mat4;
use crate::model::{build_player_meshes, part_model_matrix, BodyPart};
use crate::skin::{SkinFormat, SkinImage};

const VERTEX_SHADER: &str = r#"#version 300 es
precision mediump float;
in vec3 a_position;
in vec2 a_uv;
uniform mat4 u_mvp;
out vec2 v_uv;

void main() {
    v_uv = a_uv;
    gl_Position = u_mvp * vec4(a_position, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
uniform sampler2D u_texture;
in vec2 v_uv;
out vec4 out_color;

void main() {
    vec4 color = texture(u_texture, v_uv);
    if (color.a < 0.05) {
        discard;
    }
    out_color = color;
}
"#;

pub struct Renderer {
    gl: Gl,
    canvas: HtmlCanvasElement,
    program: WebGlProgram,
    mvp_uniform: WebGlUniformLocation,
    position_attrib: u32,
    uv_attrib: u32,
    texture: Option<WebGlTexture>,
    meshes: Vec<RenderMesh>,
}

struct RenderMesh {
    buffer: WebGlBuffer,
    vertex_count: i32,
    part: BodyPart,
    overlay: bool,
    pivot: crate::math::Vec3,
}

impl Renderer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, ViewerError> {
        let gl = canvas
            .get_context("webgl2")
            .map_err(|_| ViewerError::WebGlUnavailable)?
            .ok_or(ViewerError::WebGlUnavailable)?
            .dyn_into::<Gl>()
            .map_err(|_| ViewerError::WebGlUnavailable)?;

        let vertex_shader = compile_shader(&gl, Gl::VERTEX_SHADER, VERTEX_SHADER)?;
        let fragment_shader = compile_shader(&gl, Gl::FRAGMENT_SHADER, FRAGMENT_SHADER)?;
        let program = link_program(&gl, &vertex_shader, &fragment_shader)?;
        gl.use_program(Some(&program));

        let mvp_uniform = gl
            .get_uniform_location(&program, "u_mvp")
            .ok_or(ViewerError::WebGlOperation("missing u_mvp uniform"))?;
        let texture_uniform = gl
            .get_uniform_location(&program, "u_texture")
            .ok_or(ViewerError::WebGlOperation("missing u_texture uniform"))?;
        gl.uniform1i(Some(&texture_uniform), 0);

        let position_location = gl.get_attrib_location(&program, "a_position");
        if position_location < 0 {
            return Err(ViewerError::WebGlOperation("missing a_position attribute"));
        }
        let uv_location = gl.get_attrib_location(&program, "a_uv");
        if uv_location < 0 {
            return Err(ViewerError::WebGlOperation("missing a_uv attribute"));
        }

        gl.enable(Gl::DEPTH_TEST);
        gl.enable(Gl::BLEND);
        gl.blend_func(Gl::SRC_ALPHA, Gl::ONE_MINUS_SRC_ALPHA);
        gl.clear_color(0.08, 0.09, 0.11, 1.0);

        Ok(Self {
            gl,
            canvas,
            program,
            mvp_uniform,
            position_attrib: position_location as u32,
            uv_attrib: uv_location as u32,
            texture: None,
            meshes: Vec::new(),
        })
    }

    pub fn resize(&self) {
        let dpr = web_sys::window()
            .map(|window| window.device_pixel_ratio())
            .unwrap_or(1.0)
            .max(1.0);
        let width = (f64::from(self.canvas.client_width()).max(1.0) * dpr).round() as u32;
        let height = (f64::from(self.canvas.client_height()).max(1.0) * dpr).round() as u32;
        if self.canvas.width() != width || self.canvas.height() != height {
            self.canvas.set_width(width);
            self.canvas.set_height(height);
        }
        self.gl.viewport(0, 0, width as i32, height as i32);
    }

    pub fn upload_skin(&mut self, skin: &SkinImage) -> Result<(), ViewerError> {
        let texture = self
            .gl
            .create_texture()
            .ok_or(ViewerError::TextureCreation)?;
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));
        self.gl
            .tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::NEAREST as i32);
        self.gl
            .tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::NEAREST as i32);
        self.gl
            .tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
        self.gl
            .tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
        self.gl
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                Gl::TEXTURE_2D,
                0,
                Gl::RGBA as i32,
                skin.width as i32,
                skin.height as i32,
                0,
                Gl::RGBA,
                Gl::UNSIGNED_BYTE,
                Some(&skin.rgba),
            )
            .map_err(|_| ViewerError::WebGlOperation("texture upload"))?;
        self.texture = Some(texture);
        self.rebuild_model(skin.format)?;
        Ok(())
    }

    pub fn render(
        &self,
        view: Mat4,
        projection: Mat4,
        pose: WalkPose,
        overlays_enabled: bool,
    ) -> Result<(), ViewerError> {
        self.resize();
        self.gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        self.gl.use_program(Some(&self.program));
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, self.texture.as_ref());

        for mesh in &self.meshes {
            if mesh.overlay && !overlays_enabled {
                continue;
            }
            self.draw_mesh(mesh, view, projection, pose)?;
        }
        Ok(())
    }

    fn rebuild_model(&mut self, format: SkinFormat) -> Result<(), ViewerError> {
        self.meshes.clear();
        for mesh in build_player_meshes(format) {
            let buffer = self.gl.create_buffer().ok_or(ViewerError::BufferCreation)?;
            self.gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&buffer));
            let data = Float32Array::from(mesh.vertices.as_slice());
            self.gl
                .buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &data, Gl::STATIC_DRAW);
            self.meshes.push(RenderMesh {
                buffer,
                vertex_count: mesh.vertex_count,
                part: mesh.part,
                overlay: mesh.overlay,
                pivot: mesh.pivot,
            });
        }
        Ok(())
    }

    fn draw_mesh(
        &self,
        mesh: &RenderMesh,
        view: Mat4,
        projection: Mat4,
        pose: WalkPose,
    ) -> Result<(), ViewerError> {
        self.gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&mesh.buffer));
        let stride = 5 * std::mem::size_of::<f32>() as i32;
        self.gl.enable_vertex_attrib_array(self.position_attrib);
        self.gl.vertex_attrib_pointer_with_i32(
            self.position_attrib,
            3,
            Gl::FLOAT,
            false,
            stride,
            0,
        );
        self.gl.enable_vertex_attrib_array(self.uv_attrib);
        self.gl.vertex_attrib_pointer_with_i32(
            self.uv_attrib,
            2,
            Gl::FLOAT,
            false,
            stride,
            3 * std::mem::size_of::<f32>() as i32,
        );

        let model = part_model_matrix(mesh.part, mesh.pivot, pose);
        let mvp = projection.multiply(view).multiply(model);
        self.gl
            .uniform_matrix4fv_with_f32_array(Some(&self.mvp_uniform), false, &mvp.m);
        self.gl.draw_arrays(Gl::TRIANGLES, 0, mesh.vertex_count);
        Ok(())
    }
}

fn compile_shader(gl: &Gl, shader_type: u32, source: &str) -> Result<WebGlShader, ViewerError> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| ViewerError::ShaderCompile("create shader returned null".to_owned()))?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if gl
        .get_shader_parameter(&shader, Gl::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(ViewerError::ShaderCompile(
            gl.get_shader_info_log(&shader)
                .unwrap_or_else(|| "unknown shader error".to_owned()),
        ))
    }
}

fn link_program(
    gl: &Gl,
    vertex_shader: &WebGlShader,
    fragment_shader: &WebGlShader,
) -> Result<WebGlProgram, ViewerError> {
    let program = gl
        .create_program()
        .ok_or_else(|| ViewerError::ProgramLink("create program returned null".to_owned()))?;
    gl.attach_shader(&program, vertex_shader);
    gl.attach_shader(&program, fragment_shader);
    gl.link_program(&program);
    if gl
        .get_program_parameter(&program, Gl::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(ViewerError::ProgramLink(
            gl.get_program_info_log(&program)
                .unwrap_or_else(|| "unknown program link error".to_owned()),
        ))
    }
}
