// Native 3D Constellation Renderer using wgpu
// Embedded directly into egui via PaintCallback for zero-overhead native rendering

use std::collections::HashMap;
use std::sync::Arc;
use std::ops::{AddAssign, SubAssign};
use cgmath::{Matrix4, Point3, Vector3, Deg, perspective, EuclideanSpace, InnerSpace};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use crate::ConstellationNode;

pub enum NodeType {
    Feature,
    Bug,
    Roadmap,
    Decision,
    Lore,
    Architecture,
    Incident,
    Reference,
    Resource,
    TestCase,
    KnowledgeGap, // New Curiosity Node
    NeuralSignal,  // New Dopamine Node
    LatentPulse,   // New Latent Injection Node
}

/// Vertex data for 3D nodes
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct NodeVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub size: f32,
    pub activity: f32, // Pulsing factor
    pub _padding: [f32; 2],
}

/// Uniform buffer data for camera/projection
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_position: [f32; 4],
}

/// WGPU resources for 3D rendering
struct WgpuResources {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::TextureView,
    node_count: u32,
}

/// 3D Constellation Renderer state
pub struct Constellation3D {
    pub nodes: Vec<ConstellationNode>,
    pub positions: HashMap<String, Point3<f32>>,
    pub zoom: f32,
    pub rotation: (f32, f32), // yaw, pitch
    pub camera_distance: f32,
    wgpu_resources: Option<Arc<WgpuResources>>,
    needs_rebuild: bool,
}

impl Constellation3D {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            positions: HashMap::new(),
            zoom: 1.0,
            rotation: (0.0, 0.0),
            camera_distance: 500.0,
            wgpu_resources: None,
            needs_rebuild: true,
        }
    }

    /// Add nodes from constellation data
    pub fn load_nodes(&mut self, nodes: Vec<ConstellationNode>) {
        self.nodes = nodes;
        self.needs_rebuild = true;
    }

    /// Update node positions using force-directed layout
    pub fn update_layout(&mut self, iterations: usize) {
        if self.nodes.is_empty() {
            return;
        }
        
        // Initialize positions if needed
        for node in &self.nodes {
            if !self.positions.contains_key(&node.id) {
                self.positions.insert(
                    node.id.clone(),
                    Point3::new(
                        (node.spatial_coord.x / 100.0) as f32,
                        (node.spatial_coord.y / 100.0) as f32,
                        (node.spatial_coord.z / 100.0) as f32,
                    ),
                );
            }
        }
        
        // Run force-directed layout
        let node_ids: Vec<String> = self.nodes.iter().map(|n| n.id.clone()).collect();
        let node_count = node_ids.len();
        
        for _ in 0..iterations {
            let mut forces: HashMap<String, Vector3<f32>> = HashMap::new();
            
            // Repulsion between all nodes
            for i in 0..node_count {
                for j in (i + 1)..node_count {
                    let pos_a = self.positions[&node_ids[i]];
                    let pos_b = self.positions[&node_ids[j]];
                    let diff = pos_a.to_vec() - pos_b.to_vec();
                    let dist = diff.magnitude().max(0.1);
                    let force = 50.0 / (dist * dist);
                    let force_vec = diff.normalize() * force;
                    
                    forces.entry(node_ids[i].clone()).or_insert_with(|| Vector3::new(0.0, 0.0, 0.0)).add_assign(force_vec);
                    forces.entry(node_ids[j].clone()).or_insert_with(|| Vector3::new(0.0, 0.0, 0.0)).sub_assign(force_vec);
                }
            }
            
            // Attraction along edges (simplified: connect nearby nodes)
            for i in 0..node_count {
                for j in (i + 1)..node_count {
                    let pos_a = self.positions[&node_ids[i]];
                    let pos_b = self.positions[&node_ids[j]];
                    let dist = (pos_a - pos_b).magnitude();
                    
                    if dist < 3.0 {
                        let diff = pos_b.to_vec() - pos_a.to_vec();
                        let force = (dist - 1.5) * 0.1;
                        let force_vec = diff.normalize() * force;
                        
                        forces.entry(node_ids[i].clone()).or_insert_with(|| Vector3::new(0.0, 0.0, 0.0)).add_assign(force_vec);
                        forces.entry(node_ids[j].clone()).or_insert_with(|| Vector3::new(0.0, 0.0, 0.0)).sub_assign(force_vec);
                    }
                }
            }
            
            // Apply forces
            for (id, force) in forces {
                if let Some(pos) = self.positions.get_mut(&id) {
                    *pos += force * 0.1;
                }
            }

            // SEMANTIC CLUSTERING FORCE: Pull nodes with similar latent vectors together
            for i in 0..node_count {
                for j in (i + 1)..node_count {
                    if let (Some(v_a), Some(v_b)) = (&self.nodes[i].latent_vector, &self.nodes[j].latent_vector) {
                        let sim = self.cosine_similarity_32(v_a, v_b);
                        if sim > 0.8 {
                            let pos_a = self.positions[&node_ids[i]];
                            let pos_b = self.positions[&node_ids[j]];
                            let diff = pos_b.to_vec() - pos_a.to_vec();
                            let pull_strength = (sim - 0.8) * 2.0;
                            
                            if let Some(pos) = self.positions.get_mut(&node_ids[i]) {
                                *pos += diff * 0.05 * pull_strength as f32;
                            }
                            if let Some(pos) = self.positions.get_mut(&node_ids[j]) {
                                *pos -= diff * 0.05 * pull_strength as f32;
                            }
                        }
                    }
                }
            }
        }
    }

    fn cosine_similarity_32(&self, a: &[f32; 32], b: &[f32; 32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (norm_a * norm_b).max(1e-6)
    }

    /// Build or rebuild wgpu resources
    fn build_resources(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Constellation Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_CODE.into()),
        });
        
        // Create bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Camera Bind Group Layout"),
        });
        
        // Create pipeline layout
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Constellation Pipeline Layout"),
            bind_group_layouts: &[Some(&camera_bind_group_layout)],
            immediate_size: 0,
        });
        
        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Constellation Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<NodeVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x4,
                        2 => Float32,
                        3 => Float32,
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });
        
        // Build vertex buffer from nodes
        let vertices: Vec<NodeVertex> = self.nodes.iter()
            .filter_map(|node| {
                self.positions.get(&node.id).map(|pos| {
                    let color = node_color(&node.node_type);
                    NodeVertex {
                        position: [pos.x, pos.y, pos.z],
                        color,
                        size: 8.0,
                        activity: node.activity_pulse,
                        _padding: [0.0; 2],
                    }
                })
            })
            .collect();
        
        let vertex_buffer = if vertices.is_empty() {
            // Create empty buffer if no nodes
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Empty Vertex Buffer"),
                size: std::mem::size_of::<NodeVertex>() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            })
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Node Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        };
        
        let node_count = vertices.len() as u32;
        
        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: std::mem::size_of::<CameraUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Camera Bind Group"),
        });
        
        // Create depth texture
        let depth_texture = create_depth_texture(device, config);
        
        self.wgpu_resources = Some(Arc::new(WgpuResources {
            render_pipeline,
            vertex_buffer,
            uniform_buffer,
            uniform_bind_group,
            depth_texture,
            node_count,
        }));
        
        self.needs_rebuild = false;
    }

    /// Update camera uniforms and render
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        viewport_size: (f32, f32),
    ) {
        let Some(ref resources) = self.wgpu_resources else { return };
        if resources.node_count == 0 { return; }
        
        // Update camera
        let aspect = viewport_size.0 / viewport_size.1;
        let proj = perspective(Deg(60.0), aspect, 1.0, 2000.0);
        
        let yaw = self.rotation.0;
        let pitch = self.rotation.1;
        
        let x = self.camera_distance * pitch.cos() * yaw.sin();
        let y = self.camera_distance * pitch.sin();
        let z = self.camera_distance * pitch.cos() * yaw.cos();
        
        let camera_pos = Point3::new(x, y, z);
        let view = Matrix4::look_at_rh(camera_pos, Point3::new(0.0, 0.0, 0.0), Vector3::unit_y());
        let view_proj = proj * view;
        
        let uniform = CameraUniform {
            view_proj: view_proj.into(),
            camera_position: [camera_pos.x, camera_pos.y, camera_pos.z, 1.0],
        };
        
        queue.write_buffer(&resources.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        
        // Render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Constellation 3D Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &resources.depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        
        render_pass.set_pipeline(&resources.render_pipeline);
        render_pass.set_bind_group(0, &resources.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, resources.vertex_buffer.slice(..));
        render_pass.draw(0..resources.node_count, 0..1);
    }
}

/// Create a depth texture for 3D rendering
fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Get color for a node type
fn node_color(node_type: &crate::NodeType) -> [f32; 4] {
    match node_type {
        crate::NodeType::Feature => [0.0, 0.7, 1.0, 1.0],
        crate::NodeType::Bug => [1.0, 0.3, 0.3, 1.0],
        crate::NodeType::Roadmap => [1.0, 0.8, 0.0, 1.0],
        crate::NodeType::Decision => [0.8, 0.4, 1.0, 1.0],
        crate::NodeType::Lore => [0.4, 1.0, 0.4, 1.0],
        crate::NodeType::Architecture => [0.0, 1.0, 0.8, 1.0],
        crate::NodeType::Incident => [1.0, 0.4, 0.2, 1.0],
        crate::NodeType::Reference => [0.6, 0.6, 1.0, 1.0],
        crate::NodeType::Resource => [1.0, 0.6, 0.8, 1.0],
        crate::NodeType::TestCase => [0.8, 1.0, 0.6, 1.0],
        crate::NodeType::KnowledgeGap => [1.0, 0.5, 0.0, 1.0], // Orange
        crate::NodeType::NeuralSignal => [1.0, 1.0, 0.0, 1.0], // Yellow
        crate::NodeType::LatentPulse => [0.0, 1.0, 1.0, 1.0],   // Cyan (Mathematical Thought)
    }
}

/// WGSL Shader Code for 3D constellation rendering
const SHADER_CODE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) point_size: f32,
    @location(2) activity: f32,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) size: f32,
    @location(3) activity: f32,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    out.point_size = size * (1.0 + activity * 0.5); // Activity pulses the size
    out.activity = activity;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// Callback wrapper for egui PaintCallback
pub struct ConstellationCallback {
    pub renderer: Arc<std::sync::Mutex<Constellation3D>>,
}

impl ConstellationCallback {
    pub fn new() -> Self {
        Self {
            renderer: Arc::new(std::sync::Mutex::new(Constellation3D::new())),
        }
    }
}

/// Create a paint callback for egui wgpu integration
/// This stores the callback data in the PaintCallback for later retrieval by the wgpu backend
pub fn create_paint_callback(
    rect: egui::Rect,
    callback: &ConstellationCallback,
) -> egui::PaintCallback {
    egui::PaintCallback {
        rect,
        callback: std::sync::Arc::new(PaintCallbackData {
            renderer: callback.renderer.clone(),
        }),
    }
}

/// Data stored in the PaintCallback
struct PaintCallbackData {
    renderer: Arc<std::sync::Mutex<Constellation3D>>,
}

/// Render the 3D constellation using the provided wgpu device and queue
/// This function is called from the egui paint callback
pub fn render_constellation_3d(
    callback: &ConstellationCallback,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target_format: wgpu::TextureFormat,
    viewport_size: (f32, f32),
) {
    let mut renderer = callback.renderer.lock().unwrap();
    
    // Build resources on first frame or when needed
    if renderer.wgpu_resources.is_none() || renderer.needs_rebuild {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: target_format,
            width: viewport_size.0.max(1.0) as u32,
            height: viewport_size.1.max(1.0) as u32,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        renderer.build_resources(device, &config);
        renderer.update_layout(50);
    }
    
    // Update camera uniforms
    if let Some(ref resources) = renderer.wgpu_resources {
        if resources.node_count > 0 {
            let aspect = viewport_size.0 / viewport_size.1;
            let proj = perspective(Deg(60.0), aspect, 1.0, 2000.0);
            
            let yaw = renderer.rotation.0;
            let pitch = renderer.rotation.1;
            
            let x = renderer.camera_distance * pitch.cos() * yaw.sin();
            let y = renderer.camera_distance * pitch.sin();
            let z = renderer.camera_distance * pitch.cos() * yaw.cos();
            
            let camera_pos = Point3::new(x, y, z);
            let view = Matrix4::look_at_rh(camera_pos, Point3::new(0.0, 0.0, 0.0), Vector3::unit_y());
            let view_proj = proj * view;
            
            let uniform = CameraUniform {
                view_proj: view_proj.into(),
                camera_position: [camera_pos.x, camera_pos.y, camera_pos.z, 1.0],
            };
            
            queue.write_buffer(&resources.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_3d_renderer_creation() {
        let renderer = Constellation3D::new();
        assert!(renderer.nodes.is_empty());
        assert!(renderer.positions.is_empty());
    }

    #[test]
    fn test_node_color_mapping() {
        assert_eq!(node_color(&crate::NodeType::Feature), [0.0, 0.7, 1.0, 1.0]);
        assert_eq!(node_color(&crate::NodeType::Bug), [1.0, 0.3, 0.3, 1.0]);
    }
}
