use js_sys::Float32Array;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlTexture, WebGlUniformLocation,
};

use crate::animation::WalkPose;
use crate::controls::{LightMode, ViewerPreset};
use crate::error::ViewerError;
use crate::math::{Mat4, Vec3};
use crate::model::{
    build_player_meshes_for_style, meshes_debug_bounds, meshes_part_debug_bounds,
    part_model_matrix, BodyPart, MeshDebugBounds, MeshPartDebugBounds, PlayerModelStyle,
};
use crate::skin::{ModelVariant, SkinFormat, SkinImage};

const VERTEX_SHADER: &str = r#"#version 300 es
precision mediump float;
in vec3 a_position;
in vec2 a_uv;
in vec3 a_normal;
uniform mat4 u_mvp;
uniform mat4 u_model;
out vec2 v_uv;
out vec3 v_normal;

void main() {
    v_uv = a_uv;
    v_normal = normalize(mat3(u_model) * a_normal);
    gl_Position = u_mvp * vec4(a_position, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
uniform sampler2D u_texture;
uniform bool u_force_opaque;
uniform bool u_debug_solid;
uniform bool u_unlit;
uniform float u_ambient;
uniform float u_directional;
uniform vec3 u_light_dir;
uniform vec4 u_debug_color;
in vec2 v_uv;
in vec3 v_normal;
out vec4 out_color;

void main() {
    vec4 color = u_debug_solid ? u_debug_color : texture(u_texture, v_uv);
    if (!u_debug_solid && !u_force_opaque && color.a <= 0.00001) {
        discard;
    }
    if (u_debug_solid || u_force_opaque) {
        color.a = 1.0;
    }
    if (!u_unlit) {
        float light = u_ambient + max(dot(normalize(v_normal), normalize(u_light_dir)), 0.0) * u_directional;
        color.rgb *= light;
    }
    out_color = color;
}
"#;

pub struct Renderer {
    gl: Gl,
    canvas: HtmlCanvasElement,
    program: WebGlProgram,
    mvp_uniform: WebGlUniformLocation,
    model_uniform: WebGlUniformLocation,
    force_opaque_uniform: WebGlUniformLocation,
    debug_solid_uniform: WebGlUniformLocation,
    unlit_uniform: WebGlUniformLocation,
    ambient_uniform: WebGlUniformLocation,
    directional_uniform: WebGlUniformLocation,
    light_dir_uniform: WebGlUniformLocation,
    debug_color_uniform: WebGlUniformLocation,
    position_attrib: u32,
    uv_attrib: u32,
    normal_attrib: u32,
    texture: Option<WebGlTexture>,
    cape_texture: Option<WebGlTexture>,
    meshes: Vec<RenderMesh>,
    model_bounds: Option<MeshDebugBounds>,
    model_part_bounds: Vec<MeshPartDebugBounds>,
    cape_mesh: Option<RenderMesh>,
    model_preset: ViewerPreset,
    skin_format: Option<SkinFormat>,
    model_variant: Option<ModelVariant>,
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
        let model_uniform = gl
            .get_uniform_location(&program, "u_model")
            .ok_or(ViewerError::WebGlOperation("missing u_model uniform"))?;
        let texture_uniform = gl
            .get_uniform_location(&program, "u_texture")
            .ok_or(ViewerError::WebGlOperation("missing u_texture uniform"))?;
        gl.uniform1i(Some(&texture_uniform), 0);
        let force_opaque_uniform = gl.get_uniform_location(&program, "u_force_opaque").ok_or(
            ViewerError::WebGlOperation("missing u_force_opaque uniform"),
        )?;
        let debug_solid_uniform = gl
            .get_uniform_location(&program, "u_debug_solid")
            .ok_or(ViewerError::WebGlOperation("missing u_debug_solid uniform"))?;
        let unlit_uniform = gl
            .get_uniform_location(&program, "u_unlit")
            .ok_or(ViewerError::WebGlOperation("missing u_unlit uniform"))?;
        let ambient_uniform = gl
            .get_uniform_location(&program, "u_ambient")
            .ok_or(ViewerError::WebGlOperation("missing u_ambient uniform"))?;
        let directional_uniform = gl
            .get_uniform_location(&program, "u_directional")
            .ok_or(ViewerError::WebGlOperation("missing u_directional uniform"))?;
        let light_dir_uniform = gl
            .get_uniform_location(&program, "u_light_dir")
            .ok_or(ViewerError::WebGlOperation("missing u_light_dir uniform"))?;
        let debug_color_uniform = gl
            .get_uniform_location(&program, "u_debug_color")
            .ok_or(ViewerError::WebGlOperation("missing u_debug_color uniform"))?;

        let position_location = gl.get_attrib_location(&program, "a_position");
        if position_location < 0 {
            return Err(ViewerError::WebGlOperation("missing a_position attribute"));
        }
        let uv_location = gl.get_attrib_location(&program, "a_uv");
        if uv_location < 0 {
            return Err(ViewerError::WebGlOperation("missing a_uv attribute"));
        }
        let normal_location = gl.get_attrib_location(&program, "a_normal");
        if normal_location < 0 {
            return Err(ViewerError::WebGlOperation("missing a_normal attribute"));
        }

        gl.enable(Gl::DEPTH_TEST);
        gl.blend_func(Gl::SRC_ALPHA, Gl::ONE_MINUS_SRC_ALPHA);
        gl.clear_color(0.055, 0.06, 0.075, 1.0);

        Ok(Self {
            gl,
            canvas,
            program,
            mvp_uniform,
            model_uniform,
            force_opaque_uniform,
            debug_solid_uniform,
            unlit_uniform,
            ambient_uniform,
            directional_uniform,
            light_dir_uniform,
            debug_color_uniform,
            position_attrib: position_location as u32,
            uv_attrib: uv_location as u32,
            normal_attrib: normal_location as u32,
            texture: None,
            cape_texture: None,
            meshes: Vec::new(),
            model_bounds: None,
            model_part_bounds: Vec::new(),
            cape_mesh: None,
            model_preset: ViewerPreset::Default,
            skin_format: None,
            model_variant: None,
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

    pub fn pixel_size(&self) -> (f32, f32) {
        (
            self.canvas.width().max(1) as f32,
            self.canvas.height().max(1) as f32,
        )
    }

    pub fn upload_skin(&mut self, skin: &SkinImage) -> Result<(), ViewerError> {
        let texture = self.upload_texture(skin.width, skin.height, &skin.rgba)?;
        self.texture = Some(texture);
        self.skin_format = Some(skin.format);
        self.model_variant = Some(skin.model);
        self.rebuild_model(skin)?;
        Ok(())
    }

    pub fn set_model_preset(&mut self, preset: ViewerPreset) -> Result<(), ViewerError> {
        let previous_style = model_style(self.model_preset);
        let next_style = model_style(preset);
        self.model_preset = preset;
        if previous_style == next_style {
            return Ok(());
        }

        if let (Some(format), Some(variant)) = (self.skin_format, self.model_variant) {
            self.rebuild_model_from_parts(format, variant)?;
        }
        Ok(())
    }

    pub fn upload_cape(&mut self, cape: &crate::skin::CapeImage) -> Result<(), ViewerError> {
        let texture = self.upload_texture(cape.width, cape.height, &cape.rgba)?;
        let mesh = crate::model::build_cape_mesh(cape.width as f32, cape.height as f32);
        self.cape_texture = Some(texture);
        self.cape_mesh = Some(self.create_render_mesh(mesh)?);
        Ok(())
    }

    pub fn clear_cape(&mut self) {
        self.cape_texture = None;
        self.cape_mesh = None;
    }

    pub fn render(
        &self,
        view: Mat4,
        projection: Mat4,
        pose: WalkPose,
        overlays_enabled: bool,
        cape_visible: bool,
        unlit: bool,
        preset: ViewerPreset,
        presentation: Mat4,
        light_dir: Vec3,
    ) -> Result<(), ViewerError> {
        self.gl.use_program(Some(&self.program));
        self.set_viewer_preset(preset, light_dir);
        self.gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, self.texture.as_ref());

        self.set_debug_solid(false);
        self.set_unlit(unlit);
        self.gl.disable(Gl::CULL_FACE);
        self.gl.disable(Gl::BLEND);
        self.gl.depth_mask(true);
        self.gl
            .uniform1i(Some(&self.force_opaque_uniform), i32::from(true));
        self.gl.bind_texture(Gl::TEXTURE_2D, self.texture.as_ref());
        for mesh in &self.meshes {
            if mesh.overlay {
                continue;
            }
            self.draw_mesh(mesh, view, projection, presentation, pose)?;
        }
        if cape_visible {
            self.gl
                .uniform1i(Some(&self.force_opaque_uniform), i32::from(false));
            self.gl.disable(Gl::BLEND);
            self.gl.depth_mask(true);
        }
        if let (true, Some(texture), Some(mesh)) =
            (cape_visible, &self.cape_texture, &self.cape_mesh)
        {
            self.gl.bind_texture(Gl::TEXTURE_2D, Some(texture));
            self.draw_mesh(mesh, view, projection, presentation, pose)?;
        }
        if cape_visible {
            self.gl.depth_mask(true);
            self.gl
                .uniform1i(Some(&self.force_opaque_uniform), i32::from(true));
        }
        if overlays_enabled {
            self.gl.disable(Gl::CULL_FACE);
            self.gl.bind_texture(Gl::TEXTURE_2D, self.texture.as_ref());
            self.gl
                .uniform1i(Some(&self.force_opaque_uniform), i32::from(false));
            self.gl.enable(Gl::BLEND);
            self.gl.enable(Gl::POLYGON_OFFSET_FILL);
            self.gl.polygon_offset(1.0, 1.0);
            self.gl.depth_mask(false);
            for mesh in &self.meshes {
                if !mesh.overlay {
                    continue;
                }
                self.draw_mesh(mesh, view, projection, presentation, pose)?;
            }
            self.gl.depth_mask(true);
            self.gl.disable(Gl::POLYGON_OFFSET_FILL);
            self.gl.disable(Gl::BLEND);
        }
        self.gl.disable(Gl::CULL_FACE);
        Ok(())
    }

    pub fn render_head_debug(
        &self,
        view: Mat4,
        projection: Mat4,
        pose: WalkPose,
        textured: bool,
        overlay: bool,
    ) -> Result<(), ViewerError> {
        self.gl.use_program(Some(&self.program));
        self.set_viewer_preset(ViewerPreset::Default, Vec3::new(0.0, 0.0, 1.0));
        self.gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, self.texture.as_ref());
        self.gl.disable(Gl::BLEND);
        self.gl.depth_mask(true);
        self.set_unlit(true);
        self.set_debug_solid(!textured);
        self.gl
            .uniform1i(Some(&self.force_opaque_uniform), i32::from(true));

        for mesh in &self.meshes {
            if mesh.part == BodyPart::Head && !mesh.overlay {
                self.draw_mesh(mesh, view, projection, Mat4::identity(), pose)?;
            }
        }

        if overlay {
            self.set_debug_solid(false);
            self.gl
                .uniform1i(Some(&self.force_opaque_uniform), i32::from(false));
            self.gl.enable(Gl::BLEND);
            self.gl.enable(Gl::POLYGON_OFFSET_FILL);
            self.gl.polygon_offset(1.0, 1.0);
            self.gl.depth_mask(false);
            for mesh in &self.meshes {
                if mesh.part == BodyPart::Head && mesh.overlay {
                    self.draw_mesh(mesh, view, projection, Mat4::identity(), pose)?;
                }
            }
            self.gl.depth_mask(true);
            self.gl.disable(Gl::POLYGON_OFFSET_FILL);
            self.gl.disable(Gl::BLEND);
        }

        self.set_debug_solid(false);
        self.set_unlit(false);
        Ok(())
    }

    fn rebuild_model(&mut self, skin: &SkinImage) -> Result<(), ViewerError> {
        self.rebuild_model_from_parts(skin.format, skin.model)
    }

    fn rebuild_model_from_parts(
        &mut self,
        format: SkinFormat,
        variant: ModelVariant,
    ) -> Result<(), ViewerError> {
        self.meshes.clear();
        let meshes = build_player_meshes_for_style(format, variant, model_style(self.model_preset));
        self.model_bounds = Some(meshes_debug_bounds(meshes.iter()));
        self.model_part_bounds = meshes_part_debug_bounds(meshes.iter());
        for mesh in meshes {
            let render_mesh = self.create_render_mesh(mesh)?;
            self.meshes.push(render_mesh);
        }
        Ok(())
    }

    pub fn model_bounds(&self) -> Option<MeshDebugBounds> {
        self.model_bounds
    }

    pub fn model_part_bounds(&self) -> &[MeshPartDebugBounds] {
        &self.model_part_bounds
    }

    fn upload_texture(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<WebGlTexture, ViewerError> {
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
                width as i32,
                height as i32,
                0,
                Gl::RGBA,
                Gl::UNSIGNED_BYTE,
                Some(rgba),
            )
            .map_err(|_| ViewerError::WebGlOperation("texture upload"))?;
        Ok(texture)
    }

    fn create_render_mesh(&self, mesh: crate::model::Mesh) -> Result<RenderMesh, ViewerError> {
        let buffer = self.gl.create_buffer().ok_or(ViewerError::BufferCreation)?;
        self.gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&buffer));
        let data = Float32Array::from(mesh.vertices.as_slice());
        self.gl
            .buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &data, Gl::STATIC_DRAW);
        Ok(RenderMesh {
            buffer,
            vertex_count: mesh.vertex_count,
            part: mesh.part,
            overlay: mesh.overlay,
            pivot: mesh.pivot,
        })
    }

    fn set_debug_solid(&self, enabled: bool) {
        self.gl
            .uniform1i(Some(&self.debug_solid_uniform), i32::from(enabled));
        self.gl
            .uniform4f(Some(&self.debug_color_uniform), 0.35, 0.82, 0.96, 1.0);
    }

    fn set_unlit(&self, enabled: bool) {
        self.gl
            .uniform1i(Some(&self.unlit_uniform), i32::from(enabled));
    }

    fn set_viewer_preset(&self, preset: ViewerPreset, camera_light_dir: Vec3) {
        let (clear, ambient, directional, light_dir) = match preset.light_mode() {
            LightMode::Default => ([0.055, 0.06, 0.075, 1.0], 0.70, 0.35, camera_light_dir),
        };

        self.gl.clear_color(clear[0], clear[1], clear[2], clear[3]);
        self.gl.uniform1f(Some(&self.ambient_uniform), ambient);
        self.gl
            .uniform1f(Some(&self.directional_uniform), directional);
        self.gl.uniform3f(
            Some(&self.light_dir_uniform),
            light_dir.x,
            light_dir.y,
            light_dir.z,
        );
    }

    fn draw_mesh(
        &self,
        mesh: &RenderMesh,
        view: Mat4,
        projection: Mat4,
        presentation: Mat4,
        pose: WalkPose,
    ) -> Result<(), ViewerError> {
        self.gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&mesh.buffer));
        let stride = crate::model::VERTEX_STRIDE as i32 * std::mem::size_of::<f32>() as i32;
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
        self.gl.enable_vertex_attrib_array(self.normal_attrib);
        self.gl.vertex_attrib_pointer_with_i32(
            self.normal_attrib,
            3,
            Gl::FLOAT,
            false,
            stride,
            5 * std::mem::size_of::<f32>() as i32,
        );

        let model = presentation.multiply(part_model_matrix(mesh.part, mesh.pivot, pose));
        let mvp = projection.multiply(view).multiply(model);
        self.gl
            .uniform_matrix4fv_with_f32_array(Some(&self.model_uniform), false, &model.m);
        self.gl
            .uniform_matrix4fv_with_f32_array(Some(&self.mvp_uniform), false, &mvp.m);
        self.gl.draw_arrays(Gl::TRIANGLES, 0, mesh.vertex_count);
        Ok(())
    }
}

fn model_style(preset: ViewerPreset) -> PlayerModelStyle {
    if preset.uses_skinview3d_hierarchy() {
        PlayerModelStyle::Skinview3d1To1
    } else {
        PlayerModelStyle::FeetAtY0
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
