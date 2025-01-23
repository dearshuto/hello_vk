use winit::{application::ApplicationHandler, event::StartCause, event_loop::ActiveEventLoop};

fn main() {
    winit::event_loop::EventLoop::new()
        .unwrap()
        .run_app(&mut App::new())
        .unwrap();
}

struct App {
    window: Option<winit::window::Window>,

    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    queue: ash::vk::Queue,
    // surface_instance: ash::khr::surface::Instance,
    // surface: ash::vk::SurfaceKHR,
    // swapchain: ash::vk::SwapchainKHR,
    // swapchain_loader: ash::khr::swapchain::Device,
    // swapchain_image_view: [ash::vk::ImageView; 2],
    // command_pool: ash::vk::CommandPool,
    // command_buffer: ash::vk::CommandBuffer,
    // acquire_next_image_semaphore: ash::vk::Semaphore,
    // command_semaphore: ash::vk::Semaphore,
    // shader_object_device: ash::ext::shader_object::Device,
    // shader_objects: [ash::vk::ShaderEXT; 2],
}

impl App {
    pub fn new() -> Self {
        let entry = ash::Entry::linked();

        let instance = {
            let extension_names = [
                ash::ext::debug_utils::NAME.as_ptr(),
                ash::khr::get_physical_device_properties2::NAME.as_ptr(),
                ash::khr::portability_enumeration::NAME.as_ptr(),
            ];
            let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
                ash::vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
            } else {
                ash::vk::InstanceCreateFlags::default()
            };
            let layer_names = [
                c"VK_LAYER_KHRONOS_validation".as_ptr(),
                c"VK_LAYER_KHRONOS_shader_object".as_ptr(),
            ];
            let application_info =
                ash::vk::ApplicationInfo::default().api_version(ash::vk::API_VERSION_1_2);
            let create_info = ash::vk::InstanceCreateInfo::default()
                .application_info(&application_info)
                .enabled_extension_names(&extension_names)
                .enabled_layer_names(&layer_names)
                .flags(create_flags);
            unsafe { entry.create_instance(&create_info, None) }
        }
        .unwrap();

        let physical_device = unsafe { instance.enumerate_physical_devices() }.unwrap()[0];
        let extensions =
            unsafe { instance.enumerate_device_extension_properties(physical_device) }.unwrap();

        let is_contains = extensions.iter().any(|x| {
            let Ok(name) = x.extension_name_as_c_str() else {
                return false;
            };

            if name.to_str().unwrap() != "VK_EXT_shader_object" {
                return false;
            }

            true
        });
        if !is_contains {
            println!("required extension VK_EXT_shader_object not supported");
            panic!();
        }

        let device = {
            let mut features =
                ash::vk::PhysicalDeviceShaderObjectFeaturesEXT::default().shader_object(true);
            let enabled_extension_names = [
                ash::vk::EXT_SHADER_OBJECT_NAME.as_ptr(),
                ash::vk::KHR_DYNAMIC_RENDERING_NAME.as_ptr(),
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                ash::khr::portability_subset::NAME.as_ptr(),
            ];
            // let enabled_extension_names = [];
            let properties = [1.0];
            let queue_create_infos = [ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(0)
                .queue_priorities(&properties)];
            let create_info = ash::vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&enabled_extension_names)
                .push_next(&mut features);
            unsafe { instance.create_device(physical_device, &create_info, None) }
        }
        .unwrap();

        let queue = unsafe { device.get_device_queue(0, 0) };

        // let shader_object_device = ash::ext::shader_object::Device::new(&instance, &device);

        // let shader_create_info = [
        //     ash::vk::ShaderCreateInfoEXT::default().flags(ash::vk::ShaderCreateFlagsEXT::LINK_STAGE)
        // ];
        // let shader_object = unsafe {
        //     shader_object_device
        //         .create_shaders(&shader_create_info, None)
        //         .unwrap()
        // };

        App {
            window: None,
            entry,
            instance,
            device,
            queue,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = winit::window::WindowAttributes::default()
            .with_resizable(false)
            .with_inner_size(winit::dpi::PhysicalSize::new(1280, 960));
        let window = event_loop.create_window(window_attributes).unwrap();
        self.window = Some(window);

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::Resized(_) => {}
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}
