#![feature(proc_macro_hygiene)]
#![feature(asm)]

mod config;
mod modpack;
mod self_update;

use crate::self_update::UpdateCheckResult;
use hyperbeam_rtdx::input;
use hyperbeam_rtdx::modpack::ModpackMetadata;
use hyperbeam_unity::{reflect, texture_helpers, IlString};
use image;
use modpack::{Modpack, ModpackLoadResult};
use pmdrtdx_bindings::*;
use self_update::UpdateCheckReceiver;
use skyline::nn;
use skyline::{hook, install_hook, install_hooks};
use std::cmp::{Eq, PartialEq};
use std::ffi::CString;
use std::mem;
use std::os::raw::c_char;
use std::ptr::null_mut;
use std::string::String;

#[derive(Debug)]
enum State {
    Initializing,
    UpdateCheck(UpdateCheckReceiver),
    ModpackSelect,
    PreLoadingAnimation,
    Loading,
    Loaded,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }
}
impl Eq for State {}

struct Globals<'a> {
    state: State,
    native_plugin_manager: *mut NativePluginManager,
    script_data_manager: *mut ScriptDataManager,
    game_flow_data_manager: *mut GameFlowDataManager,
    launcher_ui: *mut GameObject,
    main_container: *mut GameObject,
    pending_operation_bg: *mut GameObject,
    launcher_animation: *mut Animation,
    title_text: *mut TMP_Text,
    version_bg: *mut GameObject,
    version_text: *mut TMP_Text,
    vanilla_icon: *mut Texture2D,
    splash_image: *mut Texture2D,
    modpacks: Vec<ModpackLoadResult>,
    loaded_modpack: Option<&'a Modpack>,
    icons: [(*mut GameObject, *mut RawImage); 7],
    selection_index: i32,
}

static mut GLOBALS: Globals = Globals {
    state: State::Initializing,
    native_plugin_manager: null_mut(),
    script_data_manager: null_mut(),
    game_flow_data_manager: null_mut(),
    launcher_ui: null_mut(),
    main_container: null_mut(),
    pending_operation_bg: null_mut(),
    launcher_animation: null_mut(),
    title_text: null_mut(),
    version_bg: null_mut(),
    version_text: null_mut(),
    vanilla_icon: null_mut(),
    splash_image: null_mut(),
    modpacks: Vec::new(),
    loaded_modpack: None,
    selection_index: 0,
    icons: [(null_mut(), null_mut()); 7],
};

unsafe fn init_launcher_ui() {
    let object_type = reflect::get_unity_type(Some("UnityEngine"), "Object").unwrap();
    let animation_type = reflect::get_unity_type(Some("UnityEngine"), "Animation").unwrap();
    let font_asset_type =
        reflect::get_type(Some("TMPro"), "TMP_FontAsset", "Unity.TextMeshPro").unwrap();
    let raw_image_type = reflect::get_unity_ui_type(Some("UnityEngine.UI"), "RawImage").unwrap();

    let mut shader_pack_wrapper = hyperbeam_unity::AssetBundleWrapper::new();
    shader_pack_wrapper.load_from_file("shader_pack");

    let mut ui_wrapper = hyperbeam_unity::AssetBundleWrapper::new();
    ui_wrapper.load_from_file("ui");
    let font = ui_wrapper
        .load_asset("SystemMenuFont SDF_US", font_asset_type)
        .unwrap() as *mut TMP_FontAsset;
    ui_wrapper.unload(false);

    let mut wrapper = hyperbeam_unity::AssetBundleWrapper::new();
    wrapper.load_from_full_path("rom:/hyperbeam/data/launcher_ui.ab");
    let canvas = wrapper.load_asset("LauncherUI", object_type).unwrap();

    GLOBALS.launcher_ui = Object_1_Instantiate(canvas, null_mut()) as *mut GameObject;

    let transform = GameObject_get_transform(GLOBALS.launcher_ui, null_mut());
    find_and_fix_text_meshes(transform, font);

    let main_container = Transform_Find(
        transform,
        IlString::new("MainUIContainer").as_ptr(),
        null_mut(),
    );
    GLOBALS.main_container = Component_1_get_gameObject(main_container as _, null_mut());

    let pending_op_text = Transform_Find(
        transform,
        IlString::new("BackgroundOverlay/PendingOperationText").as_ptr(),
        null_mut(),
    );
    GLOBALS.pending_operation_bg = Component_1_get_gameObject(pending_op_text as _, null_mut());

    GLOBALS.launcher_animation =
        GameObject_GetComponent(GLOBALS.launcher_ui as _, animation_type as _, null_mut())
            as *mut Animation;

    for i in 0..7 {
        let container_find_path = format!("MainUIContainer/LaunchOptions/LaunchOption{}", i);
        let image_find_path = format!(
            "MainUIContainer/LaunchOptions/LaunchOption{}/Mask/RawImage",
            i
        );

        let container_transform = Transform_Find(
            transform,
            IlString::new(container_find_path).as_ptr(),
            null_mut(),
        );
        let container = Component_1_get_gameObject(container_transform as _, null_mut());
        let launch_image_transform = Transform_Find(
            transform,
            IlString::new(image_find_path).as_ptr(),
            null_mut(),
        );
        let image =
            Component_1_GetComponent(launch_image_transform as _, raw_image_type as _, null_mut())
                as *mut RawImage;
        GLOBALS.icons[i] = (container, image);
    }

    shader_pack_wrapper.unload(false);
}

unsafe fn start_update_check() {
    GameObject_SetActive(GLOBALS.main_container, false, null_mut());
    GameObject_SetActive(GLOBALS.pending_operation_bg, true, null_mut());

    GLOBALS.state = State::UpdateCheck(self_update::start_check_self_update());
}

unsafe fn find_and_fix_text_meshes(root: *mut Transform, font: *mut TMP_FontAsset) {
    let tmp_type =
        reflect::get_type(Some("TMPro"), "TextMeshProUGUI", "Unity.TextMeshPro").unwrap();

    let text = find_text(root, "MainUIContainer/Info/Title", tmp_type);
    TMP_Text_set_font(text, font, null_mut());
    TMP_Text_set_lineSpacing(text, -30.0, null_mut());
    // For some reason, all text alignments are set to top left although they aren't in Unity. Fix them manually
    TMP_Text_set_alignment(text, TextAlignmentOptions__Enum_Center, null_mut());
    GLOBALS.title_text = text;

    let transform = Transform_Find(
        root,
        IlString::new("MainUIContainer/Info/VersionBG").as_ptr(),
        null_mut(),
    );
    GLOBALS.version_bg = Component_1_get_gameObject(transform as _, null_mut());

    let text = find_text(root, "MainUIContainer/Info/VersionBG/Version", tmp_type);
    TMP_Text_set_font(text, font, null_mut());
    TMP_Text_set_alignment(text, TextAlignmentOptions__Enum_Center, null_mut());
    GLOBALS.version_text = text;

    let text = find_text(root, "BackgroundOverlay/PendingOperationText", tmp_type);
    TMP_Text_set_font(text, font, null_mut());
    TMP_Text_set_alignment(text, TextAlignmentOptions__Enum_Center, null_mut());

    let text = find_text(root, "MainUIContainer/Footer/Layout/SelectText", tmp_type);
    TMP_Text_set_font(text, font, null_mut());
    TMP_Text_set_alignment(text, TextAlignmentOptions__Enum_Left, null_mut());

    let text = find_text(root, "MainUIContainer/Footer/Layout/ConfirmText", tmp_type);
    TMP_Text_set_font(text, font, null_mut());
    TMP_Text_set_alignment(text, TextAlignmentOptions__Enum_Left, null_mut());

    let text = find_text(root, "MainUIContainer/TitleBg/Text", tmp_type);
    TMP_Text_set_font(text, font, null_mut());
    TMP_Text_set_alignment(text, TextAlignmentOptions__Enum_Left, null_mut());

    let text = find_text(root, "ErrorOverlay/ErrorOverlayInner/ErrorText", tmp_type);
    TMP_Text_set_font(text, font, null_mut());
    TMP_Text_set_alignment(text, TextAlignmentOptions__Enum_Center, null_mut());
}

unsafe fn find_text(
    root: *mut Transform,
    path: &str,
    text_mesh_pro_type: &mut Type,
) -> *mut TMP_Text {
    let transform = Transform_Find(root, IlString::new(path).as_ptr(), null_mut());
    Component_1_GetComponent(transform as _, text_mesh_pro_type as _, null_mut()) as *mut TMP_Text
}

unsafe fn show_splash_image() {
    let modpack = get_current_modpack();
    if modpack.is_none() {
        return;
    }

    let modpack = modpack.unwrap();
    let raw_image_type = reflect::get_unity_ui_type(Some("UnityEngine.UI"), "RawImage").unwrap();

    let transform = GameObject_get_transform(GLOBALS.launcher_ui, null_mut());
    let splash_image_transform =
        Transform_Find(transform, IlString::new("SplashImage").as_ptr(), null_mut());
    if splash_image_transform.is_null() {
        return;
    }

    let splash_image_component =
        Component_1_GetComponent(splash_image_transform as _, raw_image_type as _, null_mut())
            as *mut RawImage;
    match modpack.load_splash_image() {
        Ok(splash_image) => {
            RawImage_set_texture(
                splash_image_component,
                splash_image.as_ptr() as _,
                null_mut(),
            );
            GLOBALS.splash_image = splash_image.as_ptr();
        }
        Err(error) => eprintln!(
            "[hyperbeam-launcher] Failed to load splash image: {}",
            error
        ),
    }
}

unsafe fn show_selected_modpack() {
    let (title_string, version_string) = match GLOBALS.selection_index {
        0 => (
            format!("Pokémon Mystery Dungeon Rescue Team DX\nNintendo"),
            None,
        ),
        i => {
            let load_result = &GLOBALS.modpacks[GLOBALS.selection_index as usize - 1];

            match load_result {
                ModpackLoadResult::Success(modpack) => (
                    format!("{}\n{}", &modpack.metadata.name, &modpack.metadata.author),
                    Some(format!("Ver. {}", modpack.metadata.version.to_string())),
                ),
                ModpackLoadResult::Invalid(invalid_modpack) => {
                    let folder_name = invalid_modpack
                        .path
                        .file_name()
                        .and_then(std::ffi::OsStr::to_str)
                        .unwrap_or_default();
                    (format!("Broken modpack\n{}", folder_name), None)
                }
            }
        }
    };
    TMP_Text_set_text(
        GLOBALS.title_text,
        IlString::new(title_string).as_ptr(),
        null_mut(),
    );
    match version_string {
        Some(version_string) => {
            TMP_Text_set_text(
                GLOBALS.version_text,
                IlString::new(version_string).as_ptr(),
                null_mut(),
            );
            GameObject_SetActive(GLOBALS.version_bg, true, null_mut());
        }
        None => {
            GameObject_SetActive(GLOBALS.version_bg, false, null_mut());
        }
    }

    for (i, (container, icon)) in GLOBALS.icons.iter_mut().enumerate() {
        let modpack_index = i as i32 - 3 + GLOBALS.selection_index;

        if modpack_index < 0 || modpack_index > GLOBALS.modpacks.len() as i32 {
            GameObject_SetActive(*container, false, null_mut());
            continue;
        }
        GameObject_SetActive(*container, true, null_mut());

        if modpack_index == 0 {
            RawImage_set_texture(*icon, GLOBALS.vanilla_icon as _, null_mut());
            continue;
        }

        let texture = match &mut GLOBALS.modpacks[modpack_index as usize - 1] {
            ModpackLoadResult::Success(modpack) => {
                let icon = modpack.load_icon();
                match icon {
                    Ok(texture) => texture.as_ptr(),
                    Err(error) => {
                        eprintln!("[hyperbeam-launcher] Failed to load icon: {}", error);
                        null_mut()
                    }
                }
            }
            _ => null_mut(),
        };
        RawImage_set_texture(*icon, texture as _, null_mut());
    }
}

fn get_current_modpack() -> Option<&'static Modpack> {
    unsafe {
        if GLOBALS.selection_index == 0 {
            return None;
        }
        if let Some(ModpackLoadResult::Success(loaded_modpack)) =
            &GLOBALS.modpacks.get(GLOBALS.selection_index as usize - 1)
        {
            Some(loaded_modpack)
        } else {
            None
        }
    }
}

fn selected_modpack_loadable() -> bool {
    unsafe { GLOBALS.selection_index == 0 || get_current_modpack().is_some() }
}

unsafe fn load_modpack(modpack: &'static Modpack) {
    println!("[hyperbeam-launcher] Loading modpack: {:?}", modpack);
    println!("[hyperbeam-launcher] Loading modpack plugins...");
    modpack.load_plugins();
    GLOBALS.loaded_modpack = Some(&modpack);
}

unsafe fn load_game() {
    let plugin_manager_start_func =
        core::mem::transmute::<_, extern "C" fn(*mut NativePluginManager)>(
            hook_native_plugin_manager_start_skyline_internal_original_fn as *const (),
        );
    plugin_manager_start_func(GLOBALS.native_plugin_manager);

    let script_data_manager_on_enable_func =
        core::mem::transmute::<_, extern "C" fn(*mut ScriptDataManager)>(
            hook_script_data_manager_on_enable_skyline_internal_original_fn as *const (),
        );
    script_data_manager_on_enable_func(GLOBALS.script_data_manager);

    let game_flow_data_manager_on_enable_func =
        core::mem::transmute::<_, extern "C" fn(*mut GameFlowDataManager)>(
            hook_game_flow_data_manager_on_enable_skyline_internal_original_fn as *const (),
        );
    game_flow_data_manager_on_enable_func(GLOBALS.game_flow_data_manager);
}

#[hook(replace = NativePluginManager_Start)]
unsafe fn hook_native_plugin_manager_start(this_ptr: *mut NativePluginManager) {
    GLOBALS.native_plugin_manager = this_ptr;
    let logo_image = image::io::Reader::open("rom:/hyperbeam/data/vanilla_icon.png")
        .expect("vanilla_icon.png missing")
        .decode()
        .unwrap()
        .flipv()
        .to_rgba8();
    GLOBALS.vanilla_icon =
        texture_helpers::texture2d_from_bytes(logo_image.as_raw(), 256, 256, false, false).as_ptr();
    init_launcher_ui();
    show_selected_modpack();
    nn::oe::FinishStartupLogo();

    start_update_check();
}

#[hook(replace = ScriptDataManager_OnEnable)]
fn hook_script_data_manager_on_enable(this_ptr: *mut ScriptDataManager) {
    println!("[hyperbeam-launcher] Prevented ScriptDataManager.OnEnable()");
    unsafe {
        GLOBALS.script_data_manager = this_ptr;
    }
}

#[hook(replace = ScriptDataStore_1_ScriptData__PreLoadData)]
unsafe fn hook_script_data_store_script_data_pre_load_data(
    this_ptr: *mut ScriptDataStore_1_ScriptData_,
    preload_path_list: *mut List_1_System_String_,
    method: *mut MethodInfo,
) {
    if GLOBALS.state == State::Loading || GLOBALS.state == State::Loaded {
        call_original!(this_ptr, preload_path_list, method);
    } else {
        println!("[hyperbeam-launcher] Prevented ScriptDataStore_1_ScriptData.PreLoadData()");
    }
}

#[hook(replace = GameFlowDataManager_OnEnable)]
fn hook_game_flow_data_manager_on_enable(this_ptr: *mut GameFlowDataManager) {
    println!("[hyperbeam-launcher] Prevented GameFlowDataManager.OnEnable()");
    unsafe {
        GLOBALS.game_flow_data_manager = this_ptr;
    }
}

#[hook(replace = SceneFlowSystem_StartMainLoop)]
fn hook_startup_sequence_main_flow(
    this_ptr: *mut StartUpSequence,
    method: *mut MethodInfo,
) -> *mut IEnumerator {
    unsafe {
        Object_1_Destroy_1(GLOBALS.launcher_ui as _, null_mut());
        Object_1_Destroy_1(GLOBALS.vanilla_icon as _, null_mut());
        if !GLOBALS.splash_image.is_null() {
            Object_1_Destroy_1(GLOBALS.splash_image as _, null_mut());
        }
        let iter = GLOBALS
            .modpacks
            .iter_mut()
            .filter_map(|modpack| match modpack {
                ModpackLoadResult::Success(modpack) => Some(modpack),
                _ => None,
            })
            .for_each(|modpack| modpack.unload_icon());
        GLOBALS.state = State::Loaded;
    }
    call_original!(this_ptr, method)
}

#[hook(replace = GroundManager_Update)]
unsafe fn hook_ground_manager_update(_this_ptr: *mut GroundManager) {
    if GLOBALS.state == State::Loaded {
        return;
    }

    il2cpp_initialize_method_metadata(0x35b9u32);

    InputSystem_Startup(null_mut());

    if GLOBALS.state != State::Initializing && GLOBALS.state != State::Loading {
        input::force_update();
    }

    let dt = Time_get_deltaTime(null_mut());

    let show_splash_image_anim_name = IlString::new("ShowSplashImage").as_ptr();
    let launcher_animation_playing =
        Animation_get_isPlaying(GLOBALS.launcher_animation, null_mut());

    match &GLOBALS.state {
        State::UpdateCheck(receiver) => {
            if let Ok(update_check_result) = receiver.try_recv() {
                match update_check_result {
                    Ok(update_check_result) => {
                        match update_check_result {
                            UpdateCheckResult::UpdateAvailable(update) => {
                                //update.start_update();
                                GLOBALS.state = State::ModpackSelect;
                                GameObject_SetActive(
                                    GLOBALS.pending_operation_bg,
                                    false,
                                    null_mut(),
                                );
                                GameObject_SetActive(GLOBALS.main_container, true, null_mut());
                            }
                            UpdateCheckResult::NoUpdate => {
                                GLOBALS.state = State::ModpackSelect;
                                GameObject_SetActive(
                                    GLOBALS.pending_operation_bg,
                                    false,
                                    null_mut(),
                                );
                                GameObject_SetActive(GLOBALS.main_container, true, null_mut());
                            }
                        };
                    }
                    Err(error) => {
                        eprintln!("Update check error: {:?}", error)
                    }
                };
            }
        }
        State::ModpackSelect => {
            if !launcher_animation_playing {
                if input::get_button(input::Button::Left) {
                    if GLOBALS.selection_index > 0 {
                        GLOBALS.selection_index -= 1;
                        show_selected_modpack();
                        Animation_Play_3(
                            GLOBALS.launcher_animation,
                            IlString::new("Left").as_ptr(),
                            null_mut(),
                        );
                    }
                }
                if input::get_button(input::Button::Right) {
                    if GLOBALS.selection_index < GLOBALS.modpacks.len() as i32 {
                        GLOBALS.selection_index += 1;
                        show_selected_modpack();
                        Animation_Play_3(
                            GLOBALS.launcher_animation,
                            IlString::new("Right").as_ptr(),
                            null_mut(),
                        );
                    }
                }
                if input::get_button_down(input::Button::A) && selected_modpack_loadable() {
                    GLOBALS.state = State::PreLoadingAnimation;
                    show_splash_image();
                    Animation_Play_3(
                        GLOBALS.launcher_animation,
                        show_splash_image_anim_name,
                        null_mut(),
                    );
                }
            }
        }
        State::PreLoadingAnimation => {
            if !launcher_animation_playing {
                GLOBALS.state = State::Loading;
                if let Some(modpack) = get_current_modpack() {
                    load_modpack(modpack);
                }
                load_game();
            }
        }
        _ => {}
    };
}

fn install_launcher_hooks() {
    install_hooks!(
        hook_script_data_manager_on_enable,
        hook_script_data_store_script_data_pre_load_data,
        hook_startup_sequence_main_flow,
        hook_game_flow_data_manager_on_enable
    );
    install_hooks!(hook_native_plugin_manager_start, hook_ground_manager_update);
}

unsafe fn auto_launch(id: &str) {
    if id == "vanilla" {
        // If we're auto-launching vanilla, nothing else needs to be done
        println!("[hyperbeam-launcher] Launching vanilla.");
        return;
    } else if let Some(ModpackLoadResult::Success(modpack)) =
        GLOBALS.modpacks.iter().find(|modpack| match modpack {
            ModpackLoadResult::Success(modpack) => &modpack.metadata.id == id,
            ModpackLoadResult::Invalid(_) => false,
        })
    {
        load_modpack(modpack);
    } else {
        // Failed to auto-launch, show UI instead
        install_launcher_hooks();
    }
}

#[skyline::main(name = "hyperbeam_launcher")]
pub unsafe fn main() {
    println!("???????????? OLD VERSION");
    let launch_config = config::get_config();
    println!(
        "[hyperbeam_launcher] Initializing with config: {:?}",
        launch_config
    );
    GLOBALS.modpacks = modpack::load_all_modpacks().expect("Failed to load modpacks!");

    if let Some(auto_launch_id) = &launch_config.auto_launch {
        auto_launch(auto_launch_id);
    } else {
        install_launcher_hooks();
    }
}

#[no_mangle]
fn hbGetCurrentModpackMetadata() -> Option<ModpackMetadata> {
    unsafe {
        GLOBALS
            .loaded_modpack
            .map(|modpack| modpack.metadata.clone())
    }
}
