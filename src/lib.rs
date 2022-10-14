#[macro_use]
extern crate log;
extern crate android_logger;
extern crate jni;

/// Expose the JNI interface for android below
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
pub mod unkprox {
    use std::net::SocketAddr;
    use std::os::fd::AsRawFd;

    use android_logger::{Config, FilterBuilder};
    use core::ffi::c_void;
    use cstr_core::CString;
    use jni::{
        objects::{JClass, JObject},
        sys::jstring,
        JNIEnv,
    };
    use log::Level;
    use socket2::{Domain, Socket, Type};

    #[no_mangle]
    static mut SOCK: i32 = 0;

    #[no_mangle]
    static mut LOCKER: libc::pthread_mutex_t = libc::PTHREAD_MUTEX_INITIALIZER;
    #[no_mangle]
    static mut SUSPENDER: libc::pthread_mutex_t = libc::PTHREAD_MUTEX_INITIALIZER;

    /// # Panics
    ///
    /// Panics if `CString::new` fails.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it dereferences a raw pointer.
    #[no_mangle]
    pub unsafe extern "C" fn Java_com_example_myktapp_MainActivity_greeting(
        env: JNIEnv,
        _: JClass,
    ) -> jstring {
        let world_ptr = CString::new("Hello world from Rust world").unwrap();
        let output = env
            .new_string(world_ptr.to_str().unwrap())
            .expect("Couldn't create java string!");
        output.into_inner()
    }

    /// Sends `size` bytes to the server from the `data` buffer
    ///
    /// # Safety
    ///
    /// Calls [`libc::pthread_mutex_lock`], [`libc::pthread_mutex_unlock`], and [`libc::send`].
    #[no_mangle]
    pub unsafe extern "C" fn sendtoServer(data: *mut c_void, size: usize) -> isize {
        libc::pthread_mutex_lock(&mut LOCKER);
        let length_sent = libc::send(SOCK, data, size, 0);
        libc::pthread_mutex_unlock(&mut LOCKER);
        length_sent
    }

    /// Receives `size` bytes from the server and stores them in `data`.
    ///
    /// # Safety
    ///
    /// Calls [`libc::pthread_mutex_lock`], [`libc::pthread_mutex_unlock`], and [`libc::recv`].
    #[no_mangle]
    pub unsafe extern "C" fn recvfromServer(data: *mut c_void, size: usize) -> isize {
        libc::pthread_mutex_lock(&mut LOCKER);
        let length_recvd = libc::recv(SOCK, data, size, 0);
        libc::pthread_mutex_unlock(&mut LOCKER);
        length_recvd
    }

    #[no_mangle]
    pub const extern "C" fn messageReceived(_: usize) -> i32 {
        1
    }

    #[no_mangle]
    pub extern "C" fn clientAwaitMessages(_null: *mut c_void) -> *mut c_void {
        info!("Unk Proxy - Await messages");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(10));

            let mut data = [0; 1];

            unsafe {
                libc::pthread_mutex_lock(&mut SUSPENDER);
                libc::pthread_mutex_lock(&mut LOCKER);

                // recv from SOCK
                let length_recvd = libc::recv(SOCK, data.as_mut_ptr().cast::<libc::c_void>(), 1, 0);
                if length_recvd > 0 {
                    info!("Unk Proxy - Message was received.");
                    messageReceived(length_recvd as usize);
                }

                libc::pthread_mutex_unlock(&mut LOCKER);
                libc::pthread_mutex_unlock(&mut SUSPENDER);
            }
        }
    }

    /// # Safety
    ///
    /// Calls [`libc::pthread_create`]
    #[no_mangle]
    pub unsafe extern "C" fn createThread() -> i32 {
        let mut thread_id: libc::pthread_t = 0;
        info!("Unk Proxy - Creating thread");

        libc::pthread_create(
            &mut thread_id,
            std::ptr::null(),
            clientAwaitMessages,
            std::ptr::null_mut(),
        )
    }

    /// # Safety
    ///
    /// Calls [`libc::pthread_mutex_lock`]
    #[no_mangle]
    pub unsafe extern "C" fn suspendThread() -> i32 {
        info!("Unk Proxy - Suspending thread");
        libc::pthread_mutex_lock(&mut SUSPENDER);
        1
    }

    /// # Safety
    ///
    /// Calls [`libc::pthread_mutex_unlock`]
    #[no_mangle]
    pub unsafe extern "C" fn resumeThread() -> i32 {
        info!("Unk Proxy - Resuming thread");
        libc::pthread_mutex_unlock(&mut SUSPENDER);
        1
    }

    /// # Panics
    ///
    /// Panics if [`Socket::new`] fails (a socket could not be created).
    ///
    /// # Safety
    ///
    /// This function is unsafe because it dereferences a raw pointer.
    #[no_mangle]
    pub unsafe extern "C" fn loadStart() -> i32 {
        info!("Unk Proxy - Starting LoadStart");

        // use socket2
        let addr: SocketAddr = include_str!(".ip_addr").parse().unwrap();
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
        socket.connect(&addr.into()).unwrap();
        SOCK = socket.as_raw_fd();

        0
    }

    /// This is effectively the starting point
    ///
    /// # Safety
    ///
    /// Calls [`libc::pthread_mutex_init`] and [`unkprox::loadStart`]
    #[no_mangle]
    pub unsafe extern "C" fn init(_: c_void) -> i32 {
        android_logger::init_once(
            Config::default()
                .with_min_level(Level::Trace) // limit log level
                .with_tag("unkprox") // logs will show under mytag tag
                .with_filter(
                    // configure messages for specific crate
                    FilterBuilder::new()
                        .parse("debug,hello::crate=error")
                        .build(),
                ),
        );

        if libc::pthread_mutex_init(&mut LOCKER, std::ptr::null()) != 0 {
            error!("Unk Proxy - Failed to init mutex");
            return -1;
        }
        if libc::pthread_mutex_init(&mut SUSPENDER, std::ptr::null()) != 0 {
            error!("Unk Proxy - Failed to init mutex");
            return -1;
        }

        info!("Unk Proxy - Mutexes Initialized");
        info!("Unk Proxy - Starting proxy connection...\n");
        self::loadStart();
        0
    }

    // #[no_mangle]
    // pub unsafe extern "C" fn get_bitmap(env: JNIEnv, _: JClass, bmp: JObject) {
    //     let mut info = graphic::AndroidBitmapInfo::new();
    //     let raw_env = env.get_native_interface();
    //
    //     let bmp = bmp.into_inner();
    //
    //     // Get bitmap info
    //     graphic::bitmap_get_info(raw_env, bmp, &mut info);
    //     let mut pixels = 0 as *mut c_void;
    // }

    /// # Safety
    ///
    /// Calls graphic methods from the Android NDK
    #[no_mangle]
    pub unsafe extern "C" fn get_bitmap(env: JNIEnv, _: JClass, bmp: JObject) -> *mut u8 {
        let mut info = graphic::AndroidBitmapInfo::new();
        let raw_env = env.get_native_interface();

        let bmp = bmp.into_inner();

        // Get bitmap info
        graphic::bitmap_get_info(raw_env, bmp, &mut info);
        let mut pixels = std::ptr::null_mut::<c_void>();

        // Lock the bitmap
        graphic::bitmap_lock_pixels(raw_env, bmp, &mut pixels);

        // Get the pixels
        let pixels = pixels as *mut u8;
        // let pixels = std::slice::from_raw_parts_mut(pixels, (info.width * info.height) as usize);

        // Unlock the bitmap
        graphic::bitmap_unlock_pixels(raw_env, bmp);

        // Return the pixels
        pixels
    }

    pub mod graphic {
        use core::ffi::{c_int, c_uint, c_void};

        use jni::sys::jobject;

        #[repr(C)]
        #[derive(Debug, Default)]
        pub struct AndroidBitmapInfo {
            pub width: c_uint,
            pub height: c_uint,
            pub stride: c_uint,
            pub format: c_int,
            pub flags: c_uint,
        }

        impl AndroidBitmapInfo {
            #[no_mangle]
            pub fn new() -> Self {
                Self::default()
            }
        }

        #[link(name = "jnigraphics", kind = "dylib")]
        extern "C" {
            #[link_name = "AndroidBitmap_getInfo"]
            pub fn bitmap_get_info(
                env: *mut jni::sys::JNIEnv,
                bmp: jobject,
                info: *mut AndroidBitmapInfo,
            ) -> c_int;

            #[link_name = "AndroidBitmap_lockPixels"]
            pub fn bitmap_lock_pixels(
                env: *mut jni::sys::JNIEnv,
                bmp: jobject,
                pixels: *mut *mut c_void,
            ) -> c_int;

            #[link_name = "AndroidBitmap_unlockPixels"]
            pub fn bitmap_unlock_pixels(env: *mut jni::sys::JNIEnv, bmp: jobject) -> c_int;
        }
    }
}
