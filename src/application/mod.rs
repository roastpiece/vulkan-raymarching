use std::collections::HashSet;
use std::env::join_paths;
use std::f32::consts::PI;
use std::ops::{Add, RangeInclusive};
use std::sync::Arc;
use std::time::Instant;
use event::WindowEvent;
use nalgebra::{Matrix, Matrix4, Rotation3, UnitQuaternion, Vector, Vector3};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::{swapchain, Validated};
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::{DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::layout::{PipelineLayoutCreateInfo, PushConstantRange};
use vulkano::render_pass::Subpass;
use vulkano::shader::{EntryPoint, ShaderStages};
use vulkano::swapchain::{SwapchainCreateInfo, SwapchainPresentInfo};
use winit::event::{DeviceEvent, ElementState, Event, MouseButton, RawKeyEvent};
use winit::window::{CursorGrabMode, Window};
use crate::{render_core, window};
use crate::render_core::vulkano_core::window_size_dependent_setup;
use vulkano::sync::GpuFuture;
use winit::event;
use winit::keyboard::{KeyCode, PhysicalKey};

pub fn run() {
    let (window, event_loop) = window::init();
    let (instance, surface) = render_core::vulkano_core::init(&event_loop, window.clone());
    let (device, queue) = render_core::vulkano_core::init_device(instance);
    let (mut swapchain, images) = render_core::vulkano_core::init_swapchain(device.clone(), surface.clone());
    let render_pass = render_core::vulkano_core::init_render_pass(device.clone(), swapchain.clone());

    let mut viewport = Viewport {
        offset: [0.0, 0.0],
        extent: [0.0, 0.0],
        depth_range: RangeInclusive::new(0.0, 1.0)
    };

    let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone(), StandardCommandBufferAllocatorCreateInfo::default());
    let buffer_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(Box::new(vulkano::sync::now(device.clone())) as Box<dyn GpuFuture>);


    let vertex_shader: EntryPoint = render_core::shaders::vs_raymarching::load(device.clone())
        .expect("Failed to create vertex shader")
        .entry_point("main").unwrap();
    let fragment_shader: EntryPoint = render_core::shaders::fs_raymarching::load(device.clone())
        .expect("Failed to create frag shader")
        .entry_point("main").unwrap();

    let vertex_input_state = MyVertex::per_vertex()
        .definition(&vertex_shader.info().input_interface).unwrap();

    let stages = vec![
        PipelineShaderStageCreateInfo::new(vertex_shader),
        PipelineShaderStageCreateInfo::new(fragment_shader)
    ];



    let pipeline_layout = PipelineLayout::new(device.clone(), PipelineLayoutCreateInfo {
        push_constant_ranges: vec![PushConstantRange {
            stages: ShaderStages::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<Constants>() as u32,
        }],
        ..PipelineLayoutCreateInfo::default()
    }).unwrap();

    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

    let pipeline = GraphicsPipeline::new(
        device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            flags: Default::default(),
            stages: stages.into(),
            vertex_input_state: Some(vertex_input_state),
            viewport_state: Some(ViewportState::default()),
            multisample_state: Some(MultisampleState::default()),
            input_assembly_state: Some(InputAssemblyState::default()),
            rasterization_state: Some(RasterizationState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState {
                    blend: Some(AttachmentBlend::alpha()),
                    ..ColorBlendAttachmentState::default()
                }
            )),
            subpass: Some(subpass.into()),
            dynamic_state: [DynamicState::Viewport].into_iter().collect(),
            ..GraphicsPipelineCreateInfo::layout(pipeline_layout.clone())
        }
    ).expect("Failed to create graphics pipeline");

    let vertices = vec![
        MyVertex { position: [-1.0, -1.0] },
        MyVertex { position: [-1.0, 1.0] },
        MyVertex { position: [1.0, -1.0] },
        MyVertex { position: [1.0, 1.0] }
    ];
    let indices = vec![0u32, 1, 2, 1, 2, 3];

    let vertex_buffer = Buffer::from_iter(
        buffer_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..BufferCreateInfo::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..AllocationCreateInfo::default()
        },
        vertices
    ).expect("Failed to create vertex buffer");

    let index_buffer = Buffer::from_iter(
        buffer_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::INDEX_BUFFER,
            ..BufferCreateInfo::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..AllocationCreateInfo::default()
        },
        indices.clone()
    ).expect("Failed to create index buffer");


    let mut delta_time = 0.0;
    let mut now = Instant::now();

    let mut pressed_keys: HashSet<KeyCode> = HashSet::new();
    let mut pitch_yaw = [0.0f32, 0.0];

    let mut camera_position = Vector3::new(0.0, 1.6, -5.0);
    let camera_up = Vector3::new(0.0, 1.0, 0.0);
    let mut camera_front = Vector3::new(0.0, 0.0, 1.0);

    let mut push_constants = Constants {
        view_matrix: get_view_matrix(camera_position, camera_front, camera_up).into(),
        camera_position: [camera_position.x, camera_position.y, camera_position.z, 0.0],
        resolution: [viewport.extent[0], viewport.extent[1]],
    };

    event_loop.run(move |event, event_loop_window_target| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                event_loop_window_target.exit();
            }
            Event::DeviceEvent {
                event: DeviceEvent::Key(RawKeyEvent { physical_key: PhysicalKey::Code(kc), state, .. }),
                ..
            } => {
                match (kc, state) {
                    (KeyCode::Escape, ElementState::Pressed) => {
                        event_loop_window_target.exit();
                    }
                    (KeyCode::KeyF, ElementState::Pressed) => {
                        if window.fullscreen().is_some() {
                            window.set_fullscreen(None);
                        } else {
                            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                        }
                    }
                    (kc, ElementState::Pressed) => {
                        pressed_keys.insert(kc);
                    }
                    (kc, ElementState::Released) => {
                        pressed_keys.remove(&kc);
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                if button == MouseButton::Left && state == ElementState::Pressed {
                    window.set_cursor_grab(CursorGrabMode::Confined)
                        .or_else(|_e| window.set_cursor_grab(CursorGrabMode::Locked))
                        .unwrap();
                    window.set_cursor_visible(false);
                    window.set_cursor_position(winit::dpi::PhysicalPosition::new(
                        viewport.extent[0] as f64 / 2.0,
                        viewport.extent[1] as f64 / 2.0
                    ))
                        .unwrap();
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                let sensitivity = 0.01 * 3.0;
                pitch_yaw[0] += delta.1 as f32 * sensitivity;
                pitch_yaw[1] -= delta.0 as f32 * sensitivity;

                if pitch_yaw[0] > 89.0 {
                    pitch_yaw[0] = 89.0;
                } else if pitch_yaw[0] < -89.0 {
                    pitch_yaw[0] = -89.0;
                }

                print!("{:?}\r", pitch_yaw);

                let direction = Vector3::new(
                    pitch_yaw[1].to_radians().cos() * pitch_yaw[0].to_radians().cos(),
                    pitch_yaw[0].to_radians().sin(),
                    pitch_yaw[1].to_radians().sin() * pitch_yaw[0].to_radians().cos()
                );
                camera_front = direction.normalize();
                push_constants.view_matrix = get_view_matrix(camera_position, camera_front, camera_up).into();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                recreate_swapchain = true;
            }
            Event::AboutToWait => {
                delta_time = {
                    let new_now = Instant::now();
                    let delta = new_now - now;
                    now = new_now;
                    delta.as_secs_f32()
                };
                camera_position = update_camera_position(&pressed_keys, &mut camera_position, camera_front, delta_time);
                push_constants.view_matrix = get_view_matrix(camera_position, camera_front, camera_up).into();
                push_constants.camera_position = [camera_position.x, camera_position.y, camera_position.z, 0.0];
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                previous_frame_end.as_mut().unwrap().cleanup_finished();
                if recreate_swapchain {
                    let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
                    let extent: [u32; 2] = window.inner_size().into();
                    push_constants.resolution = [extent[0] as f32, extent[1] as f32];

                    let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                        image_extent: extent,
                        ..swapchain.create_info()
                    }) {
                        Ok(r) => r,
                        Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                    };
                    swapchain = new_swapchain;
                    framebuffers = window_size_dependent_setup(&new_images, render_pass.clone(), &mut viewport);
                    recreate_swapchain = false;
                }

                let (image_index, suboptimal, swapchain_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(Validated::Error(vulkano::VulkanError::OutOfDate)) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };
                if suboptimal {
                    recreate_swapchain = true;
                }

                let clear_values = vec![Some([0.0, 0.0, 0.0, 1.0].into())];
                let mut builder = vulkano::command_buffer::AutoCommandBufferBuilder::primary(
                    &command_buffer_allocator,
                    queue.queue_family_index(),
                    CommandBufferUsage::OneTimeSubmit
                ).unwrap();

                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values,
                            ..RenderPassBeginInfo::framebuffer(
                                framebuffers[image_index as usize].clone(),
                            )
                        },
                        SubpassBeginInfo {
                            contents: SubpassContents::Inline,
                            ..SubpassBeginInfo::default()
                        }
                    ).unwrap()
                    .set_viewport(0, vec![viewport.clone()].into()).unwrap()
                    .push_constants(pipeline_layout.clone(), 0, push_constants.clone()).unwrap()
                    .bind_pipeline_graphics(pipeline.clone()).unwrap()
                    .bind_vertex_buffers(0, vec![vertex_buffer.clone()]).unwrap()
                    .bind_index_buffer(index_buffer.clone()).unwrap()
                    .draw_indexed(indices.len() as u32, 1, 0, 0, 0).unwrap()
                    .end_render_pass(SubpassEndInfo::default()).unwrap();
                let command_buffer = builder.build().unwrap();

                let future = previous_frame_end.take().unwrap().join(swapchain_future)
                    .then_execute(queue.clone(), command_buffer).unwrap()
                    .then_swapchain_present(queue.clone(), SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_index))
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(Box::new(future) as Box<_>);
                    }
                    Err(Validated::Error(vulkano::VulkanError::OutOfDate)) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(Box::new(vulkano::sync::now(device.clone())) as Box<_>);
                    }
                    Err(e) => {
                        eprintln!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(Box::new(vulkano::sync::now(device.clone())) as Box<_>);
                    }
                }
            }
            _ => {}
        }
    }).expect("Event Loop failed");
}

fn update_camera_position(pressed_keys: &HashSet<KeyCode>, camera_position: &mut Vector3<f32>, camera_front: Vector3<f32>, mut delta_time: f32) -> Vector3<f32> {
    delta_time *= 10.0;
    let mut camera_position = camera_position.clone();
    for kc in pressed_keys {
        let movement = camera_front.xz().normalize() * delta_time;
        match kc {
            KeyCode::KeyW => {
                camera_position.x -= movement.x;
                camera_position.z -= movement.y;
            }
            KeyCode::KeyS => {
                camera_position.x += movement.x;
                camera_position.z += movement.y;
            }
            KeyCode::KeyA => {
                camera_position.x += movement.y;
                camera_position.z -= movement.x;
            }
            KeyCode::KeyD => {
                camera_position.x -= movement.y;
                camera_position.z += movement.x;
            }
            KeyCode::Space => {
                camera_position.y += delta_time;
            }
            KeyCode::ShiftLeft => {
                camera_position.y -= delta_time;
            }
            _ => {}
        }
    }
    return camera_position;
}

fn get_view_matrix(camera_position: Vector3<f32>, camera_front: Vector3<f32>, up: Vector3<f32>) -> Matrix4<f32> {
    let up = Vector3::new(0.0, 1.0, 0.0);
    let right = camera_front.cross(&up).normalize();
    let camera_up = right.cross(&camera_front).normalize();
    let look_at = Matrix4::new(
        right.x, camera_up.x, -camera_front.x, 0.0,
        right.y, camera_up.y, -camera_front.y, 0.0,
        right.z, camera_up.z, -camera_front.z, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
    return look_at.normalize();
    // let translate = Matrix4::new_translation(&-camera_position);
    // return (look_at*translate).normalize();
}

#[repr(C)]
#[derive(BufferContents, Vertex)]
struct MyVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}

#[repr(C)]
#[derive(BufferContents, Clone)]
struct Constants {
    view_matrix: [[f32; 4]; 4],
    camera_position: [f32; 4],
    resolution: [f32; 2],
}