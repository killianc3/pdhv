use image::GenericImageView;
use wgpu::util::DeviceExt;

use std::{cmp, f32::consts::PI, mem, ops, time};

use super::{query, window};

const FIRST_BUFFER_SIZE: u64 = 1_000_000;
const SECOND_BUFFER_SIZE: u64 = 10_000;

const GREEN: [u8; 4] = [224, 249, 225, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const BG: wgpu::Color = wgpu::Color {
    r: 243.0 / 255.0,
    g: 243.0 / 255.0,
    b: 243.0 / 255.0,
    a: 1.0,
};

pub const MIN_BOX_WIDTH: usize = 420;
pub const MIN_BOX_HEIGHT: usize = 200;
pub const BASE_DPI: f32 = 144.0;

const ACC: usize = 8;

const TX1: (Pt<f32>, Pt<f32>) = (Pt { x: 0.0, y: 0.25 }, Pt { x: 0.25, y: 0.0 });
const TX2: (Pt<f32>, Pt<f32>) = (Pt { x: 0.0, y: 0.25 }, Pt { x: 0.25, y: 0.5 });
const TX3: (Pt<f32>, Pt<f32>) = (Pt { x: 0.25, y: 0.5 }, Pt { x: 0.0, y: 0.75 });
const TX4: (Pt<f32>, Pt<f32>) = (Pt { x: 0.25, y: 1.0 }, Pt { x: 0.0, y: 0.75 });

const TX5: (Pt<f32>, Pt<f32>) = (Pt { x: 0.25, y: 0.0 }, Pt { x: 0.5, y: 0.25 });
const TX6: (Pt<f32>, Pt<f32>) = (Pt { x: 0.25, y: 0.25 }, Pt { x: 0.5, y: 0.5 });
const TX7: (Pt<f32>, Pt<f32>) = (Pt { x: 0.25, y: 0.5 }, Pt { x: 0.5, y: 0.75 });
const TX8: (Pt<f32>, Pt<f32>) = (Pt { x: 0.25, y: 0.75 }, Pt { x: 0.5, y: 1.0 });

const TX9: (Pt<f32>, Pt<f32>) = (Pt { x: 0.5, y: 0.0 }, Pt { x: 0.75, y: 0.25 });
const TX10: (Pt<f32>, Pt<f32>) = (Pt { x: 0.5, y: 0.25 }, Pt { x: 0.75, y: 0.5 });
const TX11: (Pt<f32>, Pt<f32>) = (Pt { x: 0.5, y: 0.5 }, Pt { x: 0.75, y: 0.75 });
const TX12: (Pt<f32>, Pt<f32>) = (Pt { x: 0.5, y: 0.75 }, Pt { x: 0.75, y: 1.0 });

const TX13: (Pt<f32>, Pt<f32>) = (Pt { x: 0.75, y: 0.0 }, Pt { x: 1.0, y: 0.25 });
const TX14: (Pt<f32>, Pt<f32>) = (Pt { x: 0.75, y: 0.25 }, Pt { x: 1.0, y: 0.5 });
const TX15: (Pt<f32>, Pt<f32>) = (Pt { x: 0.75, y: 0.5 }, Pt { x: 1.0, y: 0.75 });
const TX16: (Pt<f32>, Pt<f32>) = (Pt { x: 0.75, y: 0.75 }, Pt { x: 1.0, y: 1.0 });

#[derive(Debug, Copy, Clone)]
pub struct Pt<T> {
    x: T,
    y: T,
}

impl<T: ops::Add<Output = T> + Copy> ops::Add<T> for Pt<T> {
    type Output = Self;

    fn add(self, other: T) -> Self::Output {
        Self {
            x: self.x + other,
            y: self.y + other,
        }
    }
}

impl<T: ops::Add<Output = T> + Copy> ops::Add<(T, T)> for Pt<T> {
    type Output = Self;

    fn add(self, tuple: (T, T)) -> Self::Output {
        Self {
            x: self.x + tuple.0,
            y: self.y + tuple.1,
        }
    }
}

impl<T: ops::Add<Output = T> + Copy> ops::Add<Self> for Pt<T> {
    type Output = Self;

    fn add(self, tuple: Self) -> Self::Output {
        Self {
            x: self.x + tuple.x,
            y: self.y + tuple.y,
        }
    }
}

impl<T: ops::Sub<Output = T> + Copy> ops::Sub<T> for Pt<T> {
    type Output = Self;

    fn sub(self, other: T) -> Self::Output {
        Self {
            x: self.x - other,
            y: self.y - other,
        }
    }
}

macro_rules! quad {
    ($p1:expr, $p2:expr, $p3:expr, $p4:expr, $bounds:expr, $color:expr) => {{
        [
            Vertex {
                position: [
                    (($p1.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p1.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                color: $color,
            },
            Vertex {
                position: [
                    (($p2.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p2.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                color: $color,
            },
            Vertex {
                position: [
                    (($p3.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p3.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                color: $color,
            },
            Vertex {
                position: [
                    (($p1.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p1.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                color: $color,
            },
            Vertex {
                position: [
                    (($p2.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p2.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                color: $color,
            },
            Vertex {
                position: [
                    (($p4.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p4.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                color: $color,
            },
        ]
    }};
    ($p1:expr, $p2:expr, $bounds:expr, $color:expr) => {{
        let p3 = Pt { x: $p1.x, y: $p2.y };
        let p4 = Pt { x: $p2.x, y: $p1.y };
        quad!($p1, $p2, p3, p4, $bounds, $color)
    }};
    ($p1:expr, $p2:expr, $p3:expr, $p4:expr, $bounds:expr, $ptx1:expr, $ptx2:expr) => {{
        [
            TextureVertex {
                position: [
                    (($p1.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p1.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                tex_coords: [
                    ($ptx1.x * u16::MAX as f32) as u16,
                    ($ptx1.y * u16::MAX as f32) as u16,
                ],
            },
            TextureVertex {
                position: [
                    (($p2.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p2.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                tex_coords: [
                    ($ptx2.x * u16::MAX as f32) as u16,
                    ($ptx2.y * u16::MAX as f32) as u16,
                ],
            },
            TextureVertex {
                position: [
                    (($p3.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p3.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                tex_coords: [
                    ($ptx1.x * u16::MAX as f32) as u16,
                    ($ptx2.y * u16::MAX as f32) as u16,
                ],
            },
            TextureVertex {
                position: [
                    (($p1.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p1.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                tex_coords: [
                    ($ptx1.x * u16::MAX as f32) as u16,
                    ($ptx1.y * u16::MAX as f32) as u16,
                ],
            },
            TextureVertex {
                position: [
                    (($p2.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p2.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                tex_coords: [
                    ($ptx2.x * u16::MAX as f32) as u16,
                    ($ptx2.y * u16::MAX as f32) as u16,
                ],
            },
            TextureVertex {
                position: [
                    (($p4.x * u16::MAX as f32) / $bounds.x) as u16,
                    (($p4.y * u16::MAX as f32) / $bounds.y) as u16,
                ],
                tex_coords: [
                    ($ptx2.x * u16::MAX as f32) as u16,
                    ($ptx1.y * u16::MAX as f32) as u16,
                ],
            },
        ]
    }};
    ($p1:expr, $p2:expr, $bounds:expr, $ptx1:expr, $ptx2:expr) => {{
        let p3 = Pt { x: $p1.x, y: $p2.y };
        let p4 = Pt { x: $p2.x, y: $p1.y };
        quad!(p3, p4, $p1, $p2, $bounds, $ptx1, $ptx2)
    }};
}

pub struct Graphic {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    first_render_pipeline: wgpu::RenderPipeline,
    first_vertex_buffer: wgpu::Buffer,
    first_vertices_count: u32,

    second_render_pipeline: wgpu::RenderPipeline,
    second_texture_bind_group: wgpu::BindGroup,
    second_vertex_buffer: wgpu::Buffer,
    second_vertices_count: u32,

    glyph_brush: wgpu_glyph::GlyphBrush<()>,
    staging_belt: wgpu::util::StagingBelt,
}

impl Graphic {
    pub async fn new(window: &window::Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            dx12_shader_compiler: wgpu::Dx12Compiler::Dxc {
                dxil_path: None,
                dxc_path: None,
            },
        });

        let surface = unsafe { instance.create_surface(window).unwrap() };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device Descriptor"),
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Rgba8Unorm,
            width: unsafe { window.get_size().0 as u32 },
            height: unsafe { window.get_size().1 as u32 },
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![wgpu::TextureFormat::Rgba8UnormSrgb],
        };
        surface.configure(&device, &config);

        // first render pipeline
        let first_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex buffer"),
            size: FIRST_BUFFER_SIZE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let first_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let first_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let first_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&first_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &first_shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &first_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        // second render pipeline
        let second_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Second Vertex Buffer"),
            size: SECOND_BUFFER_SIZE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let second_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Second Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/texture_shader.wgsl").into()),
        });

        let texture_bytes = include_bytes!("images/texture.png");
        let texture_image = image::load_from_memory(texture_bytes).unwrap();
        let texture_rgba = texture_image.to_rgba8();
        let texture_dimensions = texture_image.dimensions();
        let texture_size = wgpu::Extent3d {
            width: texture_dimensions.0,
            height: texture_dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texture"),
            view_formats: &[config.format],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texture_rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * texture_dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(texture_dimensions.1),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Texture View"),
            format: Some(config.format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 100.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: None,
            border_color: None,
        });

        let second_texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("Texture Bind Group Layout"),
            });

        let second_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &second_texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Texture Bind Group"),
        });

        let second_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Second Render Pipeline Layout"),
                bind_group_layouts: &[&second_texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let second_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Second Render Pipeline"),
                layout: Some(&second_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &second_shader,
                    entry_point: "vs_main",
                    buffers: &[TextureVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &second_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

        let font =
            wgpu_glyph::ab_glyph::FontArc::try_from_slice(include_bytes!("font/Inter-Regular.ttf"))
                .unwrap();
        let glyph_brush =
            wgpu_glyph::GlyphBrushBuilder::using_font(font).build(&device, config.format);
        let staging_belt = wgpu::util::StagingBelt::new(1024);

        Self {
            surface,
            device,
            queue,
            config,

            first_render_pipeline,
            first_vertex_buffer,
            first_vertices_count: 0,

            second_render_pipeline,
            second_texture_bind_group,
            second_vertex_buffer,
            second_vertices_count: 0,

            glyph_brush,
            staging_belt,
        }
    }
    pub fn resize(&mut self, (width, height): (i32, i32)) {
        if width > 0 && height > 0 {
            self.config.width = width as _;
            self.config.height = height as _;
            self.surface.configure(&self.device, &self.config);
        }
    }
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Texture View"),
            format: Some(self.config.format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(BG),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.first_render_pipeline);
        render_pass.set_vertex_buffer(0, self.first_vertex_buffer.slice(..));
        render_pass.draw(0..self.first_vertices_count, 0..1);

        render_pass.set_pipeline(&self.second_render_pipeline);
        render_pass.set_bind_group(0, &self.second_texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.second_vertex_buffer.slice(..));
        render_pass.draw(0..self.second_vertices_count, 0..1);

        drop(render_pass);

        self.glyph_brush
            .draw_queued(
                &self.device,
                &mut self.staging_belt,
                &mut encoder,
                &view,
                self.config.width,
                self.config.height,
            )
            .expect("Draw queued");

        self.staging_belt.finish();
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.staging_belt.recall();

        Ok(())
    }

    pub fn update(&mut self, query_v2: &query::QueryV2, dpi: u32, cx: i32, cy: i32) {
        let bounds = Pt {
            x: self.config.width as f32,
            y: self.config.height as f32,
        };
        let scale = dpi as f32 / BASE_DPI;

        let nb_box = query_v2.counters.len();
        let nb_box_x = cmp::max(
            1,
            cmp::min(
                query_v2.counters.len(),
                bounds.x as usize / (MIN_BOX_WIDTH as f32 * (dpi as f32 / BASE_DPI)) as usize,
            ),
        );
        let nb_box_y = (nb_box as f32 / nb_box_x as f32).ceil() as usize;
        let box_height = bounds.y / nb_box_y as f32;

        let pa48 = 96.0 * scale;
        let pa44 = 88.0 * scale;
        let pa32 = 64.0 * scale;
        let pa26 = 52.0 * scale;
        let pa20 = 40.0 * scale;
        let pa18 = 36.0 * scale;
        let pa16 = 32.0 * scale;
        let pa13 = 26.0 * scale;
        let pa11 = 22.0 * scale;
        let pa10 = 20.0 * scale;
        let pa6 = 12.0 * scale;
        let pa5 = 10.0 * scale;
        let pa2and5 = 5.0 * scale;
        let pa1 = 2.0 * scale;

        let mut vrt: Vec<Vertex> = Vec::new();
        let mut vrtx: Vec<TextureVertex> = Vec::new();

        for (index, counter) in query_v2.counters.values().enumerate() {
            let (px, py) = (
                index.rem_euclid(nb_box_x) as f32,
                index.div_euclid(nb_box_x) as f32,
            );
            let box_width = bounds.x / cmp::min(nb_box_x, px as usize + nb_box - index) as f32;

            let (x1, y1, x2, y2) = (
                px * box_width + if px == 0.0 { pa16 } else { pa11 },
                py * box_height + if py == 0.0 { pa16 } else { pa11 },
                (px + 1.0) * box_width
                    - if px as usize == nb_box_x - 1 || nb_box - index == 1 {
                        pa16
                    } else {
                        pa11
                    },
                (py + 1.0) * box_height
                    - if py as usize == nb_box_y - 1 {
                        pa16
                    } else {
                        pa11
                    },
            );

            let (p1, p2, p3, p4, p5, p6) = (
                Pt { x: x1, y: y1 },
                Pt { x: x2, y: y2 },
                Pt { x: x1, y: y2 },
                Pt { x: x2, y: y1 },
                Pt {
                    x: x1,
                    y: y1 + (y2 - y1) / 2.0,
                },
                Pt {
                    x: x2,
                    y: y1 + (y2 - y1) / 2.0,
                },
            );

            vrt.extend_from_slice(&quad!(p1, p2, bounds, WHITE));
            vrt.extend_from_slice(&quad!(p1 + (pa6, pa10), p2 + (-pa44, -pa10), bounds, GREEN));
            vrt.extend_from_slice(&quad!(p5 + (0.0, -pa1), p6 + (-pa44, pa1), bounds, WHITE));

            vrtx.extend_from_slice(&quad!(p1 - pa16, p1, bounds, TX2.0, TX2.1));
            vrtx.extend_from_slice(&quad!(p2 + pa16, p2, bounds, TX4.0, TX4.1));
            vrtx.extend_from_slice(&quad!(p3 + (-pa16, pa16), p3, bounds, TX1.0, TX1.1));
            vrtx.extend_from_slice(&quad!(p4 + (pa16, -pa16), p4, bounds, TX3.0, TX3.1));

            vrtx.extend_from_slice(&quad!(p1 + (0.0, -pa16), p4, bounds, TX6.0, TX6.1));
            vrtx.extend_from_slice(&quad!(p1 + (-pa16, 0.0), p3, bounds, TX5.0, TX5.1));
            vrtx.extend_from_slice(&quad!(p3, p2 + (0.0, pa16), bounds, TX8.0, TX8.1));
            vrtx.extend_from_slice(&quad!(p4, p2 + (pa16, 0.0), bounds, TX7.0, TX7.1));

            vrtx.extend_from_slice(&quad!(p1 - pa6, p1 + pa10, bounds, TX10.0, TX10.1));
            vrtx.extend_from_slice(&quad!(
                p2 + (-pa48, -pa10),
                p2 + (-pa32, pa6),
                bounds,
                TX12.0,
                TX12.1
            ));
            vrtx.extend_from_slice(&quad!(
                p3 + (-pa6, -pa10),
                p3 + (pa10, pa6),
                bounds,
                TX9.0,
                TX9.1
            ));
            vrtx.extend_from_slice(&quad!(
                p4 + (-pa48, -pa6),
                p4 + (-pa32, pa10),
                bounds,
                TX11.0,
                TX11.1
            ));

            vrtx.extend_from_slice(&quad!(
                p1 + (-pa6, pa10),
                p3 + (pa10, -pa10),
                bounds,
                TX13.0,
                TX13.1
            ));
            vrtx.extend_from_slice(&quad!(
                p1 + (pa10, -pa6),
                p4 + (-pa48, pa10),
                bounds,
                TX14.0,
                TX14.1
            ));
            vrtx.extend_from_slice(&quad!(
                p3 + (pa10, -pa10),
                p2 + (-pa48, pa6),
                bounds,
                TX16.0,
                TX16.1
            ));
            vrtx.extend_from_slice(&quad!(
                p4 + (-pa48, pa10),
                p2 + (-pa32, -pa10),
                bounds,
                TX15.0,
                TX15.1
            ));

            let of = time::Instant::now()
                .duration_since(query_v2.last_update)
                .as_secs_f32();

            let old_range = f64_max(1.0, f64_max(counter.max[0], counter.avg[0] * 2.0));
            let new_range = f64_max(1.0, f64_max(counter.max[1], counter.avg[1] * 2.0));

            let range = old_range as f32 + (new_range - old_range) as f32 * of;

            self.glyph_brush.queue(wgpu_glyph::Section {
                screen_position: (p6.x - pa32 + pa13, bounds.y - (p2.y - pa6)),
                bounds: (pa26, pa20),
                text: vec![
                    wgpu_glyph::Text::new(&range.round().to_string()).with_scale(20.0 * scale)
                ],
                layout: wgpu_glyph::Layout::default()
                    .line_breaker(wgpu_glyph::BuiltInLineBreaker::AnyCharLineBreaker)
                    .h_align(wgpu_glyph::HorizontalAlign::Center),
            });

            self.glyph_brush.queue(wgpu_glyph::Section {
                screen_position: (p6.x - pa32 + pa13, bounds.y - (p6.y + pa5)),
                bounds: (pa26, pa20),
                text: vec![wgpu_glyph::Text::new(&(range / 2.0).round().to_string())
                    .with_scale(20.0 * scale)],
                layout: wgpu_glyph::Layout::default()
                    .line_breaker(wgpu_glyph::BuiltInLineBreaker::AnyCharLineBreaker)
                    .h_align(wgpu_glyph::HorizontalAlign::Center),
            });

            if (cx as f32) > (p1.x - pa6)
                && (cx as f32) < (p2.x + pa6)
                && (bounds.y - cy as f32) > (p1.y - pa6)
                && (bounds.y - cy as f32) < (p2.y + pa6)
            {
                self.glyph_brush.queue(wgpu_glyph::Section {
                    screen_position: (p3.x + pa18, bounds.y - (p3.y - pa18)),
                    bounds: (p6.x - pa6 - pa44 - p5.x - pa18, p2.y - pa18 - p1.y - pa18),
                    text: vec![wgpu_glyph::Text::new(
                        &String::from_utf16(counter.path.as_slice()).unwrap(),
                    )
                    .with_scale(20.0 * scale)],
                    layout: wgpu_glyph::Layout::default(),
                });
            }

            //vrt.extend_from_slice(&quad!(p1 + (pa6, pa18), p2 + (-pa44, -pa18), bounds, [255, 255, 0, 255]));

            for (ic, color) in counter
                .interpolated_curves
                .iter()
                .zip(counter.instance_colors.iter())
            {
                let step = (p6.x - pa6 - pa44 - p5.x) / (ic.n - 1) as f32; 
                let mut st = of;

                let mut a1 = Pt { 
                    x: p1.x + pa6 + st * step - of * step,
                    y: p1.y + pa18 + (ic.interpolate(st, cmp::min(ic.n - 1, st.floor() as _)) / range) * (p2.y - pa18 - p1.y - pa18),
                };
                let mut a2 = a1 + (0.0, pa2and5);

                st += 1.0 / ACC as f32;

                while st < ((ic.n - 1) as f32 + of - 0.1) {
                    let b1 = Pt { 
                        x: p1.x + pa6 + st * step - of * step,
                        y: p1.y + pa18 + (ic.interpolate(st, cmp::min(ic.n - 1, st.floor() as _)) / range) * (p2.y - pa18 - p1.y - pa18),
                    };
                    let c1 = Pt { 
                        x: p1.x + pa6 + (st + 1.0 / ACC as f32) * step - of * step,
                        y: p1.y + pa18 + (ic.interpolate(st, cmp::min(ic.n - 1, (st + 1.0 / ACC as f32).floor() as _)) / range) * (p2.y - pa18 - p1.y - pa18),
                    };

                    let ac = Pt { x: c1.x - a1.x, y: c1.y - a1.y };
                    let an = f32::atan(ac.y / ac.x) + PI / 2.0;

                    let b2 = b1 + (f32::cos(an) * pa2and5, f32::sin(an) * pa2and5);

                    vrt.extend_from_slice(&quad!(
                            a1,
                            b2,
                            a2,
                            b1,
                            bounds,
                            *color
                    ));

                    a1 = b1;
                    a2 = b2;
                    st += 1.0 / ACC as f32;
                }

                let b1 = Pt { 
                    x: p1.x + pa6 + st * step - of * step,
                    y: p1.y + pa18 + (ic.interpolate(st, cmp::min(ic.n - 1, st.floor() as _)) / range) * (p2.y - pa18 - p1.y - pa18),
                };
                let b2 = b1 + (0.0, pa2and5);

                vrt.extend_from_slice(&quad!(
                        a1,
                        b2,
                        a2,
                        b1,
                        bounds,
                        *color
                ));

                vrt.extend_from_slice(&quad!(p1, p4 + (0.0, pa10), bounds, WHITE));
                vrt.extend_from_slice(&quad!(p3, p2 + (0.0, -pa10), bounds, WHITE));
            }
        }

        if vrt.len() as u64 * Vertex::desc().array_stride > self.first_vertex_buffer.size()
            || vrtx.len() as u64 * TextureVertex::desc().array_stride
                > self.second_vertex_buffer.size()
        {
            self.first_vertex_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Updated First Vertex Buffer"),
                        contents: bytemuck::cast_slice(vrt.as_slice()),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });

            self.second_vertex_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Updated First Vertex Buffer"),
                        contents: bytemuck::cast_slice(vrtx.as_slice()),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
        } else {
            self.queue.write_buffer(
                &self.first_vertex_buffer,
                0,
                bytemuck::cast_slice(vrt.as_slice()),
            );

            self.queue.write_buffer(
                &self.second_vertex_buffer,
                0,
                bytemuck::cast_slice(vrtx.as_slice()),
            );
        }

        self.first_vertices_count = vrt.len() as u32;
        self.second_vertices_count = vrtx.len() as u32;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    pub position: [u16; 2],
    pub color: [u8; 4],
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Unorm16x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[u16; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Unorm8x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct TextureVertex {
    pub position: [u16; 2],
    pub tex_coords: [u16; 2],
}

impl TextureVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<TextureVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Unorm16x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[u16; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Unorm16x2,
                },
            ],
        }
    }
}

#[inline]
fn f64_max(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}
