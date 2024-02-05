use std::sync::Arc;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::{single_pass_renderpass, Version, VulkanLibrary};
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::image::Image;
use vulkano::image::view::ImageView;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{CompositeAlpha, Surface, SurfaceInfo, Swapchain, SwapchainCreateInfo};
use winit::event_loop::EventLoop;
use winit::window::Window;

pub(crate) fn init(event_loop: &EventLoop<()>, window: Arc<Window>) -> (Arc<Instance>, Arc<Surface>) {
    let instance = {
        let library = VulkanLibrary::new().expect("VKC: Failed to load VulkanLibrary");
        let extensions = Surface::required_extensions(&event_loop);

        Instance::new(
            library,
            InstanceCreateInfo {
                enabled_extensions: extensions,
                max_api_version: Some(Version::V1_1),
                ..InstanceCreateInfo::default()
            }
        ).expect("VKC: Failed to create Instance")
    };

    let surface = Surface::from_window(instance.clone(), window).expect("VKC: Failed to create Surface");

    return (instance, surface);
}

pub fn init_device(instance: Arc<Instance>) -> (Arc<Device>, Arc<Queue>) {
    let device_extension = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };

    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices().expect("VKC: Failed to enumerate physical devices")
        .filter(|p| p.supported_extensions().contains(&device_extension))
        .filter_map(|p| {
            p.queue_family_properties().iter().enumerate().find_map(|(i, q)| {
                if q.queue_flags.contains(QueueFlags::GRAPHICS & QueueFlags::COMPUTE) {
                    Some((p.clone(), i as u32))
                } else {
                    None
                }
            })
        })
        .min_by_key(|(p, _)| {
            match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            }
        })
        .expect("VKC: Failed to find suitable physical device");

    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            enabled_extensions: device_extension,
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..QueueCreateInfo::default()
            }],
            ..DeviceCreateInfo::default()
        }
    ).expect("VKC: Failed to create Device");

    let queue = queues.next().expect("VKC: Failed to get Queue");

    return (device, queue);
}

pub fn init_swapchain(device: Arc<Device>, surface: Arc<Surface>) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
    let capabilities = device.physical_device().surface_capabilities(&surface, SurfaceInfo::default()).expect("VKC: Failed to get surface capabilities");
    let usage = capabilities.supported_usage_flags;

    let image_format = device
        .physical_device()
        .surface_formats(&surface, SurfaceInfo::default())
        .expect("VKC: Failed to get surface formats")
        [0].0;

    let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
    let image_extent: [u32; 2] = window.inner_size().into();

    Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: capabilities.min_image_count,
            image_format,
            image_extent,
            image_usage: usage,
            composite_alpha: CompositeAlpha::Opaque,
            ..SwapchainCreateInfo::default()
        }
    ).expect("VKC: Failed to create Swapchain")
}

pub fn init_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
    single_pass_renderpass!(
        device,
        attachments: {
            color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: Store,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    ).unwrap()
}

pub fn window_size_dependent_setup(
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];

    images.iter().map(|image| {
        let view = ImageView::new_default(image.clone()).expect("VKC: Failed to create ImageView");
        Framebuffer::new(
            render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![view],
                ..FramebufferCreateInfo::default()
            }
        ).expect("VKC: Failed to create Framebuffer")
    }).collect()
}