use std::borrow::Cow;

use winit::{
    application::ApplicationHandler,
    event::StartCause,
    event_loop::ActiveEventLoop,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
};

fn main() {
    winit::event_loop::EventLoop::new()
        .unwrap()
        .run_app(&mut App::new())
        .unwrap();
}

struct Instance {
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

struct App {
    window: Option<winit::window::Window>,

    entry: ash::Entry,

    instance: Option<Instance>,
}

impl App {
    pub fn new() -> Self {
        let entry = ash::Entry::linked();

        App {
            window: None,
            entry,
            instance: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = winit::window::WindowAttributes::default()
            .with_resizable(false)
            .with_inner_size(winit::dpi::PhysicalSize::new(1280, 960));
        let window = event_loop.create_window(window_attributes).unwrap();

        let raw_window_handle = window.window_handle().unwrap().as_raw();
        let raw_display_handle = window.display_handle().unwrap().as_raw();

        self.window = Some(window);

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

        let instance = {
            let mut extension_names = ash_window::enumerate_required_extensions(raw_display_handle)
                .unwrap()
                .to_vec();
            extension_names.append(&mut vec![
                ash::ext::debug_utils::NAME.as_ptr(),
                ash::khr::get_physical_device_properties2::NAME.as_ptr(),
                ash::khr::portability_enumeration::NAME.as_ptr(),
            ]);

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
            unsafe { self.entry.create_instance(&create_info, None) }
        }
        .unwrap();

        let debug_utils_loader = ash::ext::debug_utils::Instance::new(&self.entry, &instance);

        let debug_info = ash::vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
            )
            .message_type(
                ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));
        let debug_utils =
            unsafe { debug_utils_loader.create_debug_utils_messenger(&debug_info, None) }.unwrap();

        let surface_loader = ash::khr::surface::Instance::new(&self.entry, &instance);
        let surface = unsafe {
            ash_window::create_surface(
                &self.entry,
                &instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )
        }
        .unwrap();

        let (physical_device, queue_family_index) =
            unsafe { instance.enumerate_physical_devices() }
                .unwrap()
                .iter()
                .find_map(|physical_device| {
                    unsafe {
                        instance.get_physical_device_queue_family_properties(*physical_device)
                    }
                    .iter()
                    .enumerate()
                    .find_map(|(index, info)| {
                        if !info.queue_flags.contains(ash::vk::QueueFlags::GRAPHICS) {
                            return None;
                        }

                        if !unsafe {
                            surface_loader.get_physical_device_surface_support(
                                *physical_device,
                                index as u32,
                                surface,
                            )
                        }
                        .unwrap()
                        {
                            return None;
                        }

                        Some((*physical_device, index))
                    })
                })
                .unwrap();
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
                ash::khr::swapchain::NAME.as_ptr(),
                ash::vk::EXT_SHADER_OBJECT_NAME.as_ptr(),
                ash::vk::KHR_DYNAMIC_RENDERING_NAME.as_ptr(),
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                ash::khr::portability_subset::NAME.as_ptr(),
            ];
            // let enabled_extension_names = [];
            let properties = [1.0];
            let queue_create_infos = [ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index as u32)
                .queue_priorities(&properties)];
            let create_info = ash::vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&enabled_extension_names)
                .push_next(&mut features);
            unsafe { instance.create_device(physical_device, &create_info, None) }
        }
        .unwrap();

        let queue = unsafe { device.get_device_queue(queue_family_index as u32, 0) };

        let surface_format =
            unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) }
                .unwrap()[0];

        let surface_capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
        }
        .unwrap();

        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => ash::vk::Extent2D {
                width: 640,
                height: 480,
            },
            _ => surface_capabilities.current_extent,
        };
        // スワップチェーン
        // MEMO: 物理デバイスに問い合わせて他の選択肢をとることもできるが、
        // どのプラットフォームでも動作することを期待できる FIFO 方式にしておく
        let present_mode = ash::vk::PresentModeKHR::FIFO;
        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(surface_capabilities.min_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .pre_transform(ash::vk::SurfaceTransformFlagsKHR::IDENTITY) // 回転不要
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);
        let swapchain =
            unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }.unwrap();

        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }.unwrap();
        let present_image_view: Vec<_> = swapchain_images
            .iter()
            .map(|&image| {
                let create_view_info = ash::vk::ImageViewCreateInfo::default()
                    .view_type(ash::vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(ash::vk::ComponentMapping {
                        r: ash::vk::ComponentSwizzle::R,
                        g: ash::vk::ComponentSwizzle::G,
                        b: ash::vk::ComponentSwizzle::B,
                        a: ash::vk::ComponentSwizzle::A,
                    })
                    .subresource_range(ash::vk::ImageSubresourceRange {
                        aspect_mask: ash::vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                unsafe { device.create_image_view(&create_view_info, None) }.unwrap()
            })
            .collect();

        let command_pool = {
            let command_pool_create_info = ash::vk::CommandPoolCreateInfo::default()
                .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(0);
            unsafe { device.create_command_pool(&command_pool_create_info, None) }.unwrap()
        };

        // コマンドバッファー
        let command_buffers = {
            let command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo::default()
                .command_buffer_count(2)
                .command_pool(command_pool)
                .level(ash::vk::CommandBufferLevel::PRIMARY);
            unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }.unwrap()
        };
        // let shader_object_device = ash::ext::shader_object::Device::new(&instance, &device);

        // let shader_create_info = [
        //     ash::vk::ShaderCreateInfoEXT::default().flags(ash::vk::ShaderCreateFlagsEXT::LINK_STAGE)
        // ];
        // let shader_object = unsafe {
        //     shader_object_device
        //         .create_shaders(&shader_create_info, None)
        //         .unwrap()
        // };
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

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: ash::vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: ash::vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const ash::vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> ash::vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
    );

    ash::vk::FALSE
}
