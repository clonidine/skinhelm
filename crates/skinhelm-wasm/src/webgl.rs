use js_sys::Float32Array;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlBuffer, WebGlFramebuffer, WebGlProgram,
    WebGlRenderbuffer, WebGlShader, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

use crate::animation::WalkPose;
use crate::controls::{LightMode, ViewerPreset};
use crate::error::ViewerError;
use crate::math::{Mat4, Vec3};
use crate::model::{
    build_player_meshes_for_style, meshes_debug_bounds, part_model_matrix, BodyPart,
    MeshDebugBounds, PlayerModelStyle,
};
#[cfg(debug_assertions)]
use crate::model::{meshes_part_debug_bounds, MeshPartDebugBounds};
use crate::skin::{ModelVariant, SkinFormat, SkinImage};

const SKINVIEW3D_AMBIENT_LIGHT: f32 = 1.0;
const SKINVIEW3D_CAMERA_POINT_LIGHT: f32 = 0.2;

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

const FXAA_VERTEX_SHADER: &str = r#"#version 300 es
precision mediump float;
in vec2 a_position;
out vec2 v_uv;

void main() {
    v_uv = a_position * 0.5 + 0.5;
    gl_Position = vec4(a_position, 0.0, 1.0);
}
"#;

const FXAA_FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
uniform sampler2D u_scene;
uniform vec2 u_inv_resolution;
in vec2 v_uv;
out vec4 out_color;

void main() {
    vec3 rgbNW = texture(u_scene, v_uv + vec2(-1.0, -1.0) * u_inv_resolution).xyz;
    vec3 rgbNE = texture(u_scene, v_uv + vec2(1.0, -1.0) * u_inv_resolution).xyz;
    vec3 rgbSW = texture(u_scene, v_uv + vec2(-1.0, 1.0) * u_inv_resolution).xyz;
    vec3 rgbSE = texture(u_scene, v_uv + vec2(1.0, 1.0) * u_inv_resolution).xyz;
    vec4 center = texture(u_scene, v_uv);
    vec3 rgbM = center.xyz;

    vec3 luma = vec3(0.299, 0.587, 0.114);
    float lumaNW = dot(rgbNW, luma);
    float lumaNE = dot(rgbNE, luma);
    float lumaSW = dot(rgbSW, luma);
    float lumaSE = dot(rgbSE, luma);
    float lumaM = dot(rgbM, luma);
    float lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    float lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));

    vec2 dir;
    dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
    dir.y = ((lumaNW + lumaSW) - (lumaNE + lumaSE));

    float dirReduce = max((lumaNW + lumaNE + lumaSW + lumaSE) * 0.25 * 0.03125, 0.0078125);
    float reciprocalDirMin = 1.0 / (min(abs(dir.x), abs(dir.y)) + dirReduce);
    dir = min(vec2(8.0, 8.0), max(vec2(-8.0, -8.0), dir * reciprocalDirMin)) * u_inv_resolution;

    vec3 rgbA = 0.5 * (
        texture(u_scene, v_uv + dir * (1.0 / 3.0 - 0.5)).xyz +
        texture(u_scene, v_uv + dir * (2.0 / 3.0 - 0.5)).xyz
    );
    vec3 rgbB = rgbA * 0.5 + 0.25 * (
        texture(u_scene, v_uv + dir * -0.5).xyz +
        texture(u_scene, v_uv + dir * 0.5).xyz
    );
    float lumaB = dot(rgbB, luma);

    vec3 rgb = (lumaB < lumaMin || lumaB > lumaMax) ? rgbA : rgbB;
    out_color = vec4(rgb, center.a);
}
"#;

#[cfg(debug_assertions)]
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
    if (!u_debug_solid && !u_force_opaque && color.a < 0.00001) {
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

#[cfg(not(debug_assertions))]
const FRAGMENT_SHADER: &str = r#"#version 300 es
precision mediump float;
uniform sampler2D u_texture;
uniform bool u_force_opaque;
uniform float u_ambient;
uniform float u_directional;
uniform vec3 u_light_dir;
in vec2 v_uv;
in vec3 v_normal;
out vec4 out_color;

void main() {
    vec4 color = texture(u_texture, v_uv);
    if (!u_force_opaque && color.a < 0.00001) {
        discard;
    }
    if (u_force_opaque) {
        color.a = 1.0;
    }
    float light = u_ambient + max(dot(normalize(v_normal), normalize(u_light_dir)), 0.0) * u_directional;
    color.rgb *= light;
    out_color = color;
}
"#;

pub struct Renderer {
    gl: Gl,
    canvas: HtmlCanvasElement,
    program: WebGlProgram,
    fxaa_program: WebGlProgram,
    mvp_uniform: WebGlUniformLocation,
    model_uniform: WebGlUniformLocation,
    force_opaque_uniform: WebGlUniformLocation,
    fxaa_inv_resolution_uniform: WebGlUniformLocation,
    #[cfg(debug_assertions)]
    debug_solid_uniform: WebGlUniformLocation,
    #[cfg(debug_assertions)]
    unlit_uniform: WebGlUniformLocation,
    ambient_uniform: WebGlUniformLocation,
    directional_uniform: WebGlUniformLocation,
    light_dir_uniform: WebGlUniformLocation,
    #[cfg(debug_assertions)]
    debug_color_uniform: WebGlUniformLocation,
    position_attrib: u32,
    uv_attrib: u32,
    normal_attrib: u32,
    fxaa_vao: WebGlVertexArrayObject,
    fxaa_buffer: WebGlBuffer,
    texture: Option<WebGlTexture>,
    cape_texture: Option<WebGlTexture>,
    post_process: Option<PostProcessTarget>,
    meshes: Vec<RenderMesh>,
    model_bounds: Option<MeshDebugBounds>,
    #[cfg(debug_assertions)]
    model_part_bounds: Vec<MeshPartDebugBounds>,
    cape_mesh: Option<RenderMesh>,
    model_preset: ViewerPreset,
    skin_format: Option<SkinFormat>,
    model_variant: Option<ModelVariant>,
}

struct PostProcessTarget {
    framebuffer: WebGlFramebuffer,
    color_texture: WebGlTexture,
    depth_renderbuffer: WebGlRenderbuffer,
    width: u32,
    height: u32,
}

struct RenderMesh {
    vao: WebGlVertexArrayObject,
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
        let fragment_shader = compile_shader(&gl, Gl::FRAGMENT_SHADER, FRAGMENT_SHADER)
            .inspect_err(|_| gl.delete_shader(Some(&vertex_shader)))?;
        let program = link_program(&gl, &vertex_shader, &fragment_shader).inspect_err(|_| {
            gl.delete_shader(Some(&vertex_shader));
            gl.delete_shader(Some(&fragment_shader));
        })?;
        gl.delete_shader(Some(&vertex_shader));
        gl.delete_shader(Some(&fragment_shader));
        gl.use_program(Some(&program));

        let fxaa_vertex_shader = compile_shader(&gl, Gl::VERTEX_SHADER, FXAA_VERTEX_SHADER)?;
        let fxaa_fragment_shader = compile_shader(&gl, Gl::FRAGMENT_SHADER, FXAA_FRAGMENT_SHADER)
            .inspect_err(|_| gl.delete_shader(Some(&fxaa_vertex_shader)))?;
        let fxaa_program = link_program(&gl, &fxaa_vertex_shader, &fxaa_fragment_shader)
            .inspect_err(|_| {
                gl.delete_shader(Some(&fxaa_vertex_shader));
                gl.delete_shader(Some(&fxaa_fragment_shader));
            })?;
        gl.delete_shader(Some(&fxaa_vertex_shader));
        gl.delete_shader(Some(&fxaa_fragment_shader));

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
        #[cfg(debug_assertions)]
        let debug_solid_uniform = gl
            .get_uniform_location(&program, "u_debug_solid")
            .ok_or(ViewerError::WebGlOperation("missing u_debug_solid uniform"))?;
        #[cfg(debug_assertions)]
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
        #[cfg(debug_assertions)]
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
        let fxaa_position_location = gl.get_attrib_location(&fxaa_program, "a_position");
        if fxaa_position_location < 0 {
            return Err(ViewerError::WebGlOperation(
                "missing FXAA a_position attribute",
            ));
        }
        let fxaa_scene_uniform = gl
            .get_uniform_location(&fxaa_program, "u_scene")
            .ok_or(ViewerError::WebGlOperation("missing u_scene uniform"))?;
        let fxaa_inv_resolution_uniform = gl
            .get_uniform_location(&fxaa_program, "u_inv_resolution")
            .ok_or(ViewerError::WebGlOperation(
                "missing u_inv_resolution uniform",
            ))?;

        gl.use_program(Some(&fxaa_program));
        gl.uniform1i(Some(&fxaa_scene_uniform), 0);
        gl.use_program(Some(&program));

        let fxaa_vao = gl
            .create_vertex_array()
            .ok_or(ViewerError::WebGlOperation("create FXAA vertex array"))?;
        let fxaa_buffer = gl
            .create_buffer()
            .ok_or(ViewerError::WebGlOperation("create FXAA vertex buffer"))?;
        let fullscreen_triangle: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];
        gl.bind_vertex_array(Some(&fxaa_vao));
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&fxaa_buffer));
        let data = Float32Array::from(fullscreen_triangle.as_slice());
        gl.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &data, Gl::STATIC_DRAW);
        gl.enable_vertex_attrib_array(fxaa_position_location as u32);
        gl.vertex_attrib_pointer_with_i32(
            fxaa_position_location as u32,
            2,
            Gl::FLOAT,
            false,
            2 * std::mem::size_of::<f32>() as i32,
            0,
        );
        gl.bind_vertex_array(None);

        gl.enable(Gl::DEPTH_TEST);
        gl.blend_func(Gl::SRC_ALPHA, Gl::ONE_MINUS_SRC_ALPHA);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);

        Ok(Self {
            gl,
            canvas,
            program,
            fxaa_program,
            mvp_uniform,
            model_uniform,
            force_opaque_uniform,
            fxaa_inv_resolution_uniform,
            #[cfg(debug_assertions)]
            debug_solid_uniform,
            #[cfg(debug_assertions)]
            unlit_uniform,
            ambient_uniform,
            directional_uniform,
            light_dir_uniform,
            #[cfg(debug_assertions)]
            debug_color_uniform,
            position_attrib: position_location as u32,
            uv_attrib: uv_location as u32,
            normal_attrib: normal_location as u32,
            fxaa_vao,
            fxaa_buffer,
            texture: None,
            cape_texture: None,
            post_process: None,
            meshes: Vec::new(),
            model_bounds: None,
            #[cfg(debug_assertions)]
            model_part_bounds: Vec::new(),
            cape_mesh: None,
            model_preset: ViewerPreset::Default,
            skin_format: None,
            model_variant: None,
        })
    }

    pub fn resize(&mut self) {
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
        if let Err(error) = self.rebuild_model(skin) {
            self.gl.delete_texture(Some(&texture));
            return Err(error);
        }
        self.replace_skin_texture(texture);
        self.skin_format = Some(skin.format);
        self.model_variant = Some(skin.model);
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
        let render_mesh = match self.create_render_mesh(mesh) {
            Ok(render_mesh) => render_mesh,
            Err(error) => {
                self.gl.delete_texture(Some(&texture));
                return Err(error);
            }
        };
        self.replace_cape_texture(texture);
        self.replace_cape_mesh(render_mesh);
        Ok(())
    }

    pub fn clear_cape(&mut self) {
        self.clear_cape_resources();
    }

    pub fn render(
        &mut self,
        view: Mat4,
        projection: Mat4,
        pose: WalkPose,
        overlays_enabled: bool,
        cape_visible: bool,
        #[cfg_attr(not(debug_assertions), allow(unused_variables))] unlit: bool,
        preset: ViewerPreset,
        presentation: Mat4,
        light_dir: Vec3,
    ) -> Result<(), ViewerError> {
        let (target_width, target_height) = self.begin_scene_render()?;
        self.gl.use_program(Some(&self.program));
        self.set_viewer_preset(preset, light_dir);
        self.gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, self.texture.as_ref());

        #[cfg(debug_assertions)]
        self.set_debug_solid(false);
        #[cfg(debug_assertions)]
        self.set_unlit(unlit);
        self.gl.enable(Gl::CULL_FACE);
        self.gl.cull_face(Gl::BACK);
        self.gl.disable(Gl::BLEND);
        self.gl.depth_mask(true);
        self.gl
            .uniform1i(Some(&self.force_opaque_uniform), i32::from(true));
        for mesh in &self.meshes {
            if mesh.overlay {
                continue;
            }
            self.set_skinview3d_polygon_offset(skinview3d_uses_polygon_offset(mesh));
            self.draw_mesh(mesh, view, projection, presentation, pose);
        }
        self.set_skinview3d_polygon_offset(false);
        if cape_visible {
            self.gl
                .uniform1i(Some(&self.force_opaque_uniform), i32::from(false));
            self.gl.depth_mask(true);
        }
        if let (true, Some(texture), Some(mesh)) =
            (cape_visible, &self.cape_texture, &self.cape_mesh)
        {
            self.gl.bind_texture(Gl::TEXTURE_2D, Some(texture));
            self.draw_mesh(mesh, view, projection, presentation, pose);
        }
        if cape_visible {
            self.gl.depth_mask(true);
            self.gl
                .uniform1i(Some(&self.force_opaque_uniform), i32::from(true));
        }
        if overlays_enabled {
            self.gl.bind_texture(Gl::TEXTURE_2D, self.texture.as_ref());
            self.gl
                .uniform1i(Some(&self.force_opaque_uniform), i32::from(false));
            self.gl.disable(Gl::CULL_FACE);
            self.gl.enable(Gl::BLEND);
            self.gl.blend_func(Gl::SRC_ALPHA, Gl::ONE_MINUS_SRC_ALPHA);
            self.gl.depth_mask(true);
            for mesh in &self.meshes {
                if !mesh.overlay {
                    continue;
                }
                self.set_skinview3d_polygon_offset(skinview3d_uses_polygon_offset(mesh));
                self.draw_mesh(mesh, view, projection, presentation, pose);
            }
            self.set_skinview3d_polygon_offset(false);
            self.gl.disable(Gl::BLEND);
            self.gl.depth_mask(true);
        }
        self.render_fxaa(target_width, target_height)?;
        Ok(())
    }

    #[cfg(debug_assertions)]
    pub fn render_head_debug(
        &mut self,
        view: Mat4,
        projection: Mat4,
        pose: WalkPose,
        textured: bool,
        overlay: bool,
    ) -> Result<(), ViewerError> {
        let (target_width, target_height) = self.begin_scene_render()?;
        self.gl.use_program(Some(&self.program));
        self.set_viewer_preset(ViewerPreset::Default, Vec3::new(0.0, 0.0, 1.0));
        self.gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, self.texture.as_ref());
        self.gl.enable(Gl::CULL_FACE);
        self.gl.cull_face(Gl::BACK);
        self.gl.disable(Gl::BLEND);
        self.set_skinview3d_polygon_offset(false);
        self.gl.depth_mask(true);
        self.set_unlit(true);
        self.set_debug_solid(!textured);
        self.gl
            .uniform1i(Some(&self.force_opaque_uniform), i32::from(true));

        for mesh in &self.meshes {
            if mesh.part == BodyPart::Head && !mesh.overlay {
                self.draw_mesh(mesh, view, projection, Mat4::identity(), pose);
            }
        }

        if overlay {
            self.set_debug_solid(false);
            self.gl
                .uniform1i(Some(&self.force_opaque_uniform), i32::from(false));
            self.gl.disable(Gl::CULL_FACE);
            self.gl.enable(Gl::BLEND);
            self.gl.blend_func(Gl::SRC_ALPHA, Gl::ONE_MINUS_SRC_ALPHA);
            self.set_skinview3d_polygon_offset(false);
            self.gl.depth_mask(true);
            for mesh in &self.meshes {
                if mesh.part == BodyPart::Head && mesh.overlay {
                    self.draw_mesh(mesh, view, projection, Mat4::identity(), pose);
                }
            }
            self.gl.disable(Gl::BLEND);
            self.gl.depth_mask(true);
        }

        self.set_debug_solid(false);
        self.set_unlit(false);
        self.render_fxaa(target_width, target_height)?;
        Ok(())
    }

    fn rebuild_model(&mut self, skin: &SkinImage) -> Result<(), ViewerError> {
        self.rebuild_model_from_parts(skin.format, skin.model)?;
        Ok(())
    }

    fn rebuild_model_from_parts(
        &mut self,
        format: SkinFormat,
        variant: ModelVariant,
    ) -> Result<(), ViewerError> {
        let meshes = build_player_meshes_for_style(format, variant, model_style(self.model_preset));
        let model_bounds = Some(meshes_debug_bounds(meshes.iter()));
        let model_part_bounds = self.rebuilt_model_part_bounds(&meshes);
        let mut render_meshes = Vec::with_capacity(meshes.len());
        for mesh in meshes {
            match self.create_render_mesh(mesh) {
                Ok(render_mesh) => render_meshes.push(render_mesh),
                Err(error) => {
                    for render_mesh in render_meshes {
                        self.delete_render_mesh(render_mesh);
                    }
                    return Err(error);
                }
            }
        }
        self.clear_meshes();
        self.meshes = render_meshes;
        self.model_bounds = model_bounds;
        self.set_rebuilt_model_part_bounds(model_part_bounds);
        Ok(())
    }

    #[cfg(debug_assertions)]
    fn rebuilt_model_part_bounds(&self, meshes: &[crate::model::Mesh]) -> Vec<MeshPartDebugBounds> {
        meshes_part_debug_bounds(meshes.iter())
    }

    #[cfg(not(debug_assertions))]
    fn rebuilt_model_part_bounds(&self, _meshes: &[crate::model::Mesh]) {}

    #[cfg(debug_assertions)]
    fn set_rebuilt_model_part_bounds(&mut self, bounds: Vec<MeshPartDebugBounds>) {
        self.model_part_bounds = bounds;
    }

    #[cfg(not(debug_assertions))]
    fn set_rebuilt_model_part_bounds(&mut self, _bounds: ()) {}

    pub fn model_bounds(&self) -> Option<MeshDebugBounds> {
        self.model_bounds
    }

    #[cfg(debug_assertions)]
    #[inline]
    pub fn model_part_bounds(&self) -> &[MeshPartDebugBounds] {
        &self.model_part_bounds
    }

    fn upload_texture(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<WebGlTexture, ViewerError> {
        let expected_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or(ViewerError::InvalidTextureData {
                width,
                height,
                actual_len: rgba.len(),
            })?;
        if rgba.len() != expected_len {
            return Err(ViewerError::InvalidTextureData {
                width,
                height,
                actual_len: rgba.len(),
            });
        }

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

    fn begin_scene_render(&mut self) -> Result<(u32, u32), ViewerError> {
        self.ensure_post_process_target()?;
        let target = self
            .post_process
            .as_ref()
            .ok_or(ViewerError::WebGlOperation("missing post-process target"))?;
        self.gl
            .bind_framebuffer(Gl::FRAMEBUFFER, Some(&target.framebuffer));
        self.gl
            .viewport(0, 0, target.width as i32, target.height as i32);
        Ok((target.width, target.height))
    }

    fn render_fxaa(&self, width: u32, height: u32) -> Result<(), ViewerError> {
        let target = self
            .post_process
            .as_ref()
            .ok_or(ViewerError::WebGlOperation("missing post-process target"))?;

        self.gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        self.gl.viewport(0, 0, width as i32, height as i32);
        self.gl.use_program(Some(&self.fxaa_program));
        self.gl.disable(Gl::DEPTH_TEST);
        self.gl.disable(Gl::CULL_FACE);
        self.gl.disable(Gl::BLEND);
        self.gl.disable(Gl::POLYGON_OFFSET_FILL);
        self.gl.depth_mask(false);
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl
            .bind_texture(Gl::TEXTURE_2D, Some(&target.color_texture));
        self.gl.uniform2f(
            Some(&self.fxaa_inv_resolution_uniform),
            1.0 / width.max(1) as f32,
            1.0 / height.max(1) as f32,
        );
        self.gl.bind_vertex_array(Some(&self.fxaa_vao));
        self.gl.draw_arrays(Gl::TRIANGLES, 0, 3);
        self.gl.bind_vertex_array(None);
        self.gl.depth_mask(true);
        self.gl.enable(Gl::DEPTH_TEST);
        self.gl.use_program(Some(&self.program));
        Ok(())
    }

    fn ensure_post_process_target(&mut self) -> Result<(), ViewerError> {
        let width = self.canvas.width().max(1);
        let height = self.canvas.height().max(1);
        let current_matches = self
            .post_process
            .as_ref()
            .map(|target| target.width == width && target.height == height)
            .unwrap_or(false);
        if current_matches {
            return Ok(());
        }

        self.delete_post_process_target();
        self.post_process = Some(self.create_post_process_target(width, height)?);
        Ok(())
    }

    fn create_post_process_target(
        &self,
        width: u32,
        height: u32,
    ) -> Result<PostProcessTarget, ViewerError> {
        let framebuffer = self
            .gl
            .create_framebuffer()
            .ok_or(ViewerError::WebGlOperation(
                "create post-process framebuffer",
            ))?;
        let color_texture = self
            .gl
            .create_texture()
            .ok_or(ViewerError::TextureCreation)?;
        let depth_renderbuffer =
            self.gl
                .create_renderbuffer()
                .ok_or(ViewerError::WebGlOperation(
                    "create post-process depth renderbuffer",
                ))?;

        self.gl.bind_texture(Gl::TEXTURE_2D, Some(&color_texture));
        self.gl
            .tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32);
        self.gl
            .tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);
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
                None,
            )
            .map_err(|_| ViewerError::WebGlOperation("post-process texture allocation"))?;

        self.gl
            .bind_renderbuffer(Gl::RENDERBUFFER, Some(&depth_renderbuffer));
        self.gl.renderbuffer_storage(
            Gl::RENDERBUFFER,
            Gl::DEPTH_COMPONENT24,
            width as i32,
            height as i32,
        );

        self.gl
            .bind_framebuffer(Gl::FRAMEBUFFER, Some(&framebuffer));
        self.gl.framebuffer_texture_2d(
            Gl::FRAMEBUFFER,
            Gl::COLOR_ATTACHMENT0,
            Gl::TEXTURE_2D,
            Some(&color_texture),
            0,
        );
        self.gl.framebuffer_renderbuffer(
            Gl::FRAMEBUFFER,
            Gl::DEPTH_ATTACHMENT,
            Gl::RENDERBUFFER,
            Some(&depth_renderbuffer),
        );

        let status = self.gl.check_framebuffer_status(Gl::FRAMEBUFFER);
        self.gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        self.gl.bind_renderbuffer(Gl::RENDERBUFFER, None);
        self.gl.bind_texture(Gl::TEXTURE_2D, None);

        if status != Gl::FRAMEBUFFER_COMPLETE {
            self.gl.delete_framebuffer(Some(&framebuffer));
            self.gl.delete_texture(Some(&color_texture));
            self.gl.delete_renderbuffer(Some(&depth_renderbuffer));
            return Err(ViewerError::WebGlOperation(
                "post-process framebuffer incomplete",
            ));
        }

        Ok(PostProcessTarget {
            framebuffer,
            color_texture,
            depth_renderbuffer,
            width,
            height,
        })
    }

    fn create_render_mesh(&self, mesh: crate::model::Mesh) -> Result<RenderMesh, ViewerError> {
        let vao = self
            .gl
            .create_vertex_array()
            .ok_or(ViewerError::WebGlOperation("create vertex array"))?;
        let buffer = match self.gl.create_buffer() {
            Some(buffer) => buffer,
            None => {
                self.gl.delete_vertex_array(Some(&vao));
                return Err(ViewerError::BufferCreation);
            }
        };
        let stride = crate::model::VERTEX_STRIDE as i32 * std::mem::size_of::<f32>() as i32;
        let f32_size = std::mem::size_of::<f32>() as i32;

        self.gl.bind_vertex_array(Some(&vao));
        self.gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&buffer));
        let data = Float32Array::from(mesh.vertices.as_slice());
        self.gl
            .buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &data, Gl::STATIC_DRAW);
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
            3 * f32_size,
        );
        self.gl.enable_vertex_attrib_array(self.normal_attrib);
        self.gl.vertex_attrib_pointer_with_i32(
            self.normal_attrib,
            3,
            Gl::FLOAT,
            false,
            stride,
            5 * f32_size,
        );
        self.gl.bind_vertex_array(None);

        Ok(RenderMesh {
            vao,
            buffer,
            vertex_count: mesh.vertex_count,
            part: mesh.part,
            overlay: mesh.overlay,
            pivot: mesh.pivot,
        })
    }

    fn replace_skin_texture(&mut self, texture: WebGlTexture) {
        if let Some(previous) = self.texture.replace(texture) {
            self.gl.delete_texture(Some(&previous));
        }
    }

    fn replace_cape_texture(&mut self, texture: WebGlTexture) {
        if let Some(previous) = self.cape_texture.replace(texture) {
            self.gl.delete_texture(Some(&previous));
        }
    }

    fn replace_cape_mesh(&mut self, mesh: RenderMesh) {
        if let Some(previous) = self.cape_mesh.replace(mesh) {
            self.delete_render_mesh(previous);
        }
    }

    fn clear_cape_resources(&mut self) {
        if let Some(texture) = self.cape_texture.take() {
            self.gl.delete_texture(Some(&texture));
        }
        if let Some(mesh) = self.cape_mesh.take() {
            self.delete_render_mesh(mesh);
        }
    }

    fn clear_meshes(&mut self) {
        let meshes = std::mem::take(&mut self.meshes);
        for mesh in meshes {
            self.delete_render_mesh(mesh);
        }
    }

    fn delete_post_process_target(&mut self) {
        if let Some(target) = self.post_process.take() {
            self.gl.delete_framebuffer(Some(&target.framebuffer));
            self.gl.delete_texture(Some(&target.color_texture));
            self.gl
                .delete_renderbuffer(Some(&target.depth_renderbuffer));
        }
    }

    fn delete_render_mesh(&self, mesh: RenderMesh) {
        self.gl.delete_vertex_array(Some(&mesh.vao));
        self.gl.delete_buffer(Some(&mesh.buffer));
    }

    #[inline]
    #[cfg(debug_assertions)]
    fn set_debug_solid(&self, enabled: bool) {
        self.gl
            .uniform1i(Some(&self.debug_solid_uniform), i32::from(enabled));
        self.gl
            .uniform4f(Some(&self.debug_color_uniform), 0.35, 0.82, 0.96, 1.0);
    }

    #[inline]
    #[cfg(debug_assertions)]
    fn set_unlit(&self, enabled: bool) {
        self.gl
            .uniform1i(Some(&self.unlit_uniform), i32::from(enabled));
    }

    #[inline]
    fn set_viewer_preset(&self, preset: ViewerPreset, camera_light_dir: Vec3) {
        let (clear, ambient, directional, light_dir) = match preset.light_mode() {
            LightMode::Default => (
                [0.0, 0.0, 0.0, 0.0],
                SKINVIEW3D_AMBIENT_LIGHT,
                SKINVIEW3D_CAMERA_POINT_LIGHT,
                camera_light_dir,
            ),
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
    ) {
        self.gl.bind_vertex_array(Some(&mesh.vao));
        let model = presentation.multiply(part_model_matrix(mesh.part, mesh.pivot, pose));
        let mvp = projection.multiply(view).multiply(model);
        self.gl
            .uniform_matrix4fv_with_f32_array(Some(&self.model_uniform), false, &model.m);
        self.gl
            .uniform_matrix4fv_with_f32_array(Some(&self.mvp_uniform), false, &mvp.m);
        self.gl.draw_arrays(Gl::TRIANGLES, 0, mesh.vertex_count);
        self.gl.bind_vertex_array(None);
    }

    fn set_skinview3d_polygon_offset(&self, enabled: bool) {
        if enabled {
            self.gl.enable(Gl::POLYGON_OFFSET_FILL);
            self.gl.polygon_offset(1.0, 1.0);
        } else {
            self.gl.disable(Gl::POLYGON_OFFSET_FILL);
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.clear_cape_resources();
        self.clear_meshes();
        self.delete_post_process_target();
        if let Some(texture) = self.texture.take() {
            self.gl.delete_texture(Some(&texture));
        }
        self.gl.delete_vertex_array(Some(&self.fxaa_vao));
        self.gl.delete_buffer(Some(&self.fxaa_buffer));
        self.gl.delete_program(Some(&self.program));
        self.gl.delete_program(Some(&self.fxaa_program));
    }
}

#[inline]
fn skinview3d_uses_polygon_offset(mesh: &RenderMesh) -> bool {
    matches!(
        mesh.part,
        BodyPart::RightArm | BodyPart::LeftArm | BodyPart::RightLeg | BodyPart::LeftLeg
    )
}

#[inline]
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
        let info = gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "unknown shader error".to_owned());
        gl.delete_shader(Some(&shader));
        Err(ViewerError::ShaderCompile(info))
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
        let info = gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| "unknown program link error".to_owned());
        gl.delete_program(Some(&program));
        Err(ViewerError::ProgramLink(info))
    }
}
