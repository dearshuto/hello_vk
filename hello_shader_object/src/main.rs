fn main() {
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
        let layer_names = [c"VK_LAYER_KHRONOS_validation".as_ptr()];
        let application_info =
            ash::vk::ApplicationInfo::default().api_version(ash::vk::API_VERSION_1_3);
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
        return;
    }

    let device = {
        let mut features =
            ash::vk::PhysicalDeviceShaderObjectFeaturesEXT::default().shader_object(true);
        let enabled_extension_names = [ash::vk::EXT_SHADER_OBJECT_NAME.as_ptr()];
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

    // デバイス
    unsafe { device.destroy_device(None) }

    // インスタンス
    unsafe { instance.destroy_instance(None) }
}
