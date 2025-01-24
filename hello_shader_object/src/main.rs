use std::borrow::Cow;

use include_bytes_aligned::include_bytes_aligned;
use winit::{
    application::ApplicationHandler,
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
    debug_utils_instance: ash::ext::debug_utils::Instance,
    debug_utils_messanger: ash::vk::DebugUtilsMessengerEXT,
    device: ash::Device,
    queue: ash::vk::Queue,
    surface_instance: ash::khr::surface::Instance,
    surface: ash::vk::SurfaceKHR,
    swapchain: ash::vk::SwapchainKHR,
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain_image_view: [ash::vk::ImageView; 2],
    command_pool: ash::vk::CommandPool,
    command_buffer: ash::vk::CommandBuffer,
    acquire_next_image_semaphore: ash::vk::Semaphore,
    command_semaphore: ash::vk::Semaphore,
    render_pass: ash::vk::RenderPass,
    shader_object_device: ash::ext::shader_object::Device,
    shader_objects: [ash::vk::ShaderEXT; 2],
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

    pub fn request_redraw(&mut self) {
        let Some(instance) = &self.instance else {
            return;
        };

        let device = &instance.device;
        let queue = instance.queue;
        let swapchain_loader = &instance.swapchain_loader;
        let swapchain = instance.swapchain;
        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }.unwrap();
        let present_image_views = &instance.swapchain_image_view;
        let command_buffer = instance.command_buffer;
        let acquire_next_image_semaphore = instance.acquire_next_image_semaphore;
        let render_command_semaphore = instance.command_semaphore;
        let shader_object_device = &instance.shader_object_device;
        let shader_objects = &instance.shader_objects;

        unsafe { device.queue_wait_idle(queue) }.unwrap();

        let (frame_index, _) = unsafe {
            swapchain_loader.acquire_next_image(
                swapchain,
                u64::MAX,
                acquire_next_image_semaphore,
                ash::vk::Fence::null(),
            )
        }
        .unwrap();

        let command_buffer_begin_info = ash::vk::CommandBufferBeginInfo::default()
            .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { device.begin_command_buffer(command_buffer, &command_buffer_begin_info) }.unwrap();

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                ash::vk::PipelineStageFlags::TOP_OF_PIPE,
                ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ash::vk::DependencyFlags::empty(),
                &[], // MemoryBariier
                &[], // BufferMemoryBariier
                &[ash::vk::ImageMemoryBarrier::default()
                    .image(swapchain_images[frame_index as usize])
                    .src_access_mask(ash::vk::AccessFlags::empty())
                    .dst_access_mask(ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .old_layout(ash::vk::ImageLayout::UNDEFINED)
                    .new_layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .subresource_range(
                        ash::vk::ImageSubresourceRange::default()
                            .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    )],
            )
        }

        unsafe {
            device.cmd_begin_rendering(
                command_buffer,
                &ash::vk::RenderingInfo::default()
                    .render_area(
                        ash::vk::Rect2D::default()
                            .extent(ash::vk::Extent2D::default().width(1280).height(960)),
                    )
                    .layer_count(1)
                    .color_attachments(&[ash::vk::RenderingAttachmentInfo::default()
                        .load_op(ash::vk::AttachmentLoadOp::CLEAR)
                        .store_op(ash::vk::AttachmentStoreOp::STORE)
                        .image_layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .clear_value(ash::vk::ClearValue {
                            color: ash::vk::ClearColorValue {
                                float32: [1.0, 1.0, 1.0, 1.0],
                            },
                        })
                        .image_view(present_image_views[frame_index as usize])]),
            );
        }

        unsafe {
            shader_object_device.cmd_bind_shaders(
                command_buffer,
                &[
                    ash::vk::ShaderStageFlags::VERTEX,
                    ash::vk::ShaderStageFlags::FRAGMENT,
                ],
                &[shader_objects[0], shader_objects[1]],
            );
        }

        // ステートたち
        unsafe {
            // ビューポート
            device.cmd_set_viewport_with_count(
                command_buffer,
                &[ash::vk::Viewport::default().width(1280.0).height(960.0)],
            );
            device.cmd_set_scissor_with_count(
                command_buffer,
                &[ash::vk::Rect2D::default()
                    .extent(ash::vk::Extent2D::default().width(1280).height(960))],
            );

            // ラスタライザ
            device.cmd_set_rasterizer_discard_enable(command_buffer, false);
            shader_object_device
                .cmd_set_rasterization_samples(command_buffer, ash::vk::SampleCountFlags::TYPE_1);
            shader_object_device.cmd_set_alpha_to_coverage_enable(command_buffer, false);
            shader_object_device.cmd_set_polygon_mode(command_buffer, ash::vk::PolygonMode::FILL);
            device.cmd_set_primitive_restart_enable(command_buffer, true);
            device.cmd_set_primitive_topology(
                command_buffer,
                ash::vk::PrimitiveTopology::TRIANGLE_LIST,
            );
            device.cmd_set_cull_mode(command_buffer, ash::vk::CullModeFlags::NONE);

            // 深度テスト
            device.cmd_set_depth_test_enable(command_buffer, false);
            device.cmd_set_depth_write_enable(command_buffer, false);
            device.cmd_set_stencil_test_enable(command_buffer, false);
            device.cmd_set_depth_bias_enable(command_buffer, false);

            // ブレンドステート
            shader_object_device.cmd_set_color_write_mask(
                command_buffer,
                0,
                &[ash::vk::ColorComponentFlags::RGBA],
            );
            shader_object_device.cmd_set_color_blend_enable(command_buffer, 0, &[0]);
            shader_object_device.cmd_set_sample_mask(
                command_buffer,
                ash::vk::SampleCountFlags::TYPE_1,
                &[0xFFFFFFFF],
            );

            // 頂点ステート
            shader_object_device.cmd_set_vertex_input(command_buffer, &[], &[]);
        }

        unsafe {
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
        }

        unsafe { device.cmd_end_rendering(command_buffer) };

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ash::vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                ash::vk::DependencyFlags::empty(),
                &[], // MemoryBariier
                &[], // BufferMemoryBariier
                &[ash::vk::ImageMemoryBarrier::default()
                    .image(swapchain_images[frame_index as usize])
                    .src_access_mask(ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .dst_access_mask(ash::vk::AccessFlags::empty())
                    .old_layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
                    .subresource_range(
                        ash::vk::ImageSubresourceRange::default()
                            .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    )],
            )
        }

        unsafe { device.end_command_buffer(command_buffer) }.unwrap();

        // コマンドの提出
        let wait_semaphores = [acquire_next_image_semaphore];
        let signal_semaphores = [render_command_semaphore];
        let command_buffers = [command_buffer];
        let wait_mask = [ash::vk::PipelineStageFlags::BOTTOM_OF_PIPE
            | ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let submit_info = ash::vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .signal_semaphores(&signal_semaphores)
            .wait_dst_stage_mask(&wait_mask)
            .command_buffers(&command_buffers);
        unsafe { device.queue_submit(queue, &[submit_info], ash::vk::Fence::null()) }.unwrap();

        // 表示
        let wait_semaphores = [render_command_semaphore];
        let swapchains = [swapchain];
        let indices = [frame_index];
        unsafe {
            swapchain_loader.queue_present(
                queue,
                &ash::vk::PresentInfoKHR::default()
                    .wait_semaphores(&wait_semaphores)
                    .swapchains(&swapchains)
                    .image_indices(&indices),
            )
        }
        .unwrap();
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
                ash::vk::ApplicationInfo::default().api_version(ash::vk::API_VERSION_1_3);
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

        let surface_instance = ash::khr::surface::Instance::new(&self.entry, &instance);
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
                            surface_instance.get_physical_device_surface_support(
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
            let mut shader_object_features =
                ash::vk::PhysicalDeviceShaderObjectFeaturesEXT::default().shader_object(true);
            let mut dynamic_rendering_features =
                ash::vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);
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
                .push_next(&mut shader_object_features)
                .push_next(&mut dynamic_rendering_features);
            unsafe { instance.create_device(physical_device, &create_info, None) }
        }
        .unwrap();

        let queue = unsafe { device.get_device_queue(queue_family_index as u32, 0) };

        let surface_format = unsafe {
            surface_instance.get_physical_device_surface_formats(physical_device, surface)
        }
        .unwrap()[0];

        let surface_capabilities = unsafe {
            surface_instance.get_physical_device_surface_capabilities(physical_device, surface)
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
        let command_buffer = {
            let command_buffer_allocate_info = ash::vk::CommandBufferAllocateInfo::default()
                .command_buffer_count(1)
                .command_pool(command_pool)
                .level(ash::vk::CommandBufferLevel::PRIMARY);
            unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }.unwrap()[0]
        };

        let render_pass = {
            let attatchments = [ash::vk::AttachmentDescription::default()
                .format(surface_format.format)
                .initial_layout(ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .final_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
                .load_op(ash::vk::AttachmentLoadOp::CLEAR)
                .store_op(ash::vk::AttachmentStoreOp::STORE)
                .samples(ash::vk::SampleCountFlags::TYPE_1)];
            let subpasses = [ash::vk::SubpassDescription::default()
                .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&[ash::vk::AttachmentReference {
                    attachment: 0,
                    layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                }])];
            let create_info = ash::vk::RenderPassCreateInfo::default()
                .attachments(&attatchments)
                .subpasses(&subpasses);
            unsafe { device.create_render_pass(&create_info, None) }.unwrap()
        };

        let shader_object_device = ash::ext::shader_object::Device::new(&instance, &device);

        let shader_create_info = [
            ash::vk::ShaderCreateInfoEXT::default()
                // .flags(ash::vk::ShaderCreateFlagsEXT::LINK_STAGE)
                .stage(ash::vk::ShaderStageFlags::VERTEX)
                .next_stage(ash::vk::ShaderStageFlags::FRAGMENT)
                .code_type(ash::vk::ShaderCodeTypeEXT::SPIRV)
                .code(include_bytes_aligned!(4, "../res/triangle.vs.spv"))
                .name(c"main"),
            ash::vk::ShaderCreateInfoEXT::default()
                //.flags(ash::vk::ShaderCreateFlagsEXT::LINK_STAGE)
                .stage(ash::vk::ShaderStageFlags::FRAGMENT)
                .next_stage(ash::vk::ShaderStageFlags::empty())
                .code_type(ash::vk::ShaderCodeTypeEXT::SPIRV)
                .code(include_bytes_aligned!(4, "../res/triangle.fs.spv"))
                .name(c"main"),
        ];

        let shader_object = unsafe {
            shader_object_device
                .create_shaders(&shader_create_info, None)
                .unwrap()
        };

        let (acquire_next_image_semaphore, command_semaphore) = {
            let create_info = ash::vk::SemaphoreCreateInfo::default();
            let acquire_next_image_semaphore =
                unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            let command_semaphore = unsafe { device.create_semaphore(&create_info, None) }.unwrap();
            (acquire_next_image_semaphore, command_semaphore)
        };

        self.instance = Some(Instance {
            instance,
            debug_utils_instance: debug_utils_loader,
            debug_utils_messanger: debug_utils,
            device,
            queue,
            surface_instance,
            surface,
            swapchain,
            swapchain_loader,
            swapchain_image_view: [present_image_view[0], present_image_view[1]],
            command_pool,
            command_buffer,
            render_pass,
            acquire_next_image_semaphore,
            command_semaphore,
            shader_object_device,
            shader_objects: [shader_object[0], shader_object[1]],
        });
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }

        match event {
            winit::event::WindowEvent::Resized(_) => {}
            winit::event::WindowEvent::RedrawRequested => self.request_redraw(),
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let Some(instance) = &self.instance else {
            return;
        };

        // GPU の処理待ち
        unsafe { instance.device.queue_wait_idle(instance.queue) }.unwrap();

        unsafe {
            instance
                .device
                .destroy_semaphore(instance.command_semaphore, None);
            instance
                .device
                .destroy_semaphore(instance.acquire_next_image_semaphore, None);
        }

        // シェーダーオブジェクト
        unsafe {
            instance
                .shader_object_device
                .destroy_shader(instance.shader_objects[0], None);
            instance
                .shader_object_device
                .destroy_shader(instance.shader_objects[1], None);
        }

        // レンダーパス
        unsafe {
            instance
                .device
                .destroy_render_pass(instance.render_pass, None)
        }

        // コマンドバッファー
        unsafe {
            instance
                .device
                .free_command_buffers(instance.command_pool, &[instance.command_buffer]);
        }

        // コマンドプール
        unsafe {
            instance
                .device
                .destroy_command_pool(instance.command_pool, None)
        }

        // イメージ
        for image in instance.swapchain_image_view {
            unsafe { instance.device.destroy_image_view(image, None) }
        }

        // スワップチェーン
        unsafe {
            instance
                .swapchain_loader
                .destroy_swapchain(instance.swapchain, None)
        }

        // サーフェイス
        unsafe {
            instance
                .surface_instance
                .destroy_surface(instance.surface, None)
        }

        // デバッグ情報
        unsafe {
            instance
                .debug_utils_instance
                .destroy_debug_utils_messenger(instance.debug_utils_messanger, None)
        }

        // デバイス
        unsafe { instance.device.destroy_device(None) }

        // インスタンス
        unsafe { instance.instance.destroy_instance(None) }
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
