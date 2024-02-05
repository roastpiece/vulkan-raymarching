use std::ops::RangeInclusive;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::{swapchain, Validated};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents, SubpassEndInfo};
use vulkano::swapchain::{SwapchainCreateInfo, SwapchainPresentInfo};
use winit::event::Event;
use winit::window::Window;
use crate::{render_core, window};
use crate::render_core::vulkano_core::window_size_dependent_setup;
use vulkano::sync::GpuFuture;

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
    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(Box::new(vulkano::sync::now(device.clone())) as Box<dyn vulkano::sync::GpuFuture>);

    event_loop.run(move |event, event_loop_window_target| {
        match event {
            Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => {
                event_loop_window_target.exit();
            }
            Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(_),
                ..
            } => {
                recreate_swapchain = true;
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            Event::WindowEvent {
                event: winit::event::WindowEvent::RedrawRequested,
                ..
            } => {
                previous_frame_end.as_mut().unwrap().cleanup_finished();
                if recreate_swapchain {
                    let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
                    let extent: [u32; 2] = window.inner_size().into();

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

                let clear_values = vec![Some([0.0, 0.0, 1.0, 1.0].into())];
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