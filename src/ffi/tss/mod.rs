#[cfg(unix)] mod pthread;
#[cfg(unix)] pub use self::pthread::PthreadTssPtr as TssPtr;

#[cfg(windows)] mod win;
#[cfg(windows)] pub use self::win::WinTssPtr as TssPtr;

#[test]
fn test_tss_ptr_1() {
    use std::ptr;

    let ptr = TssPtr::new().unwrap();
    assert_eq!(ptr.get(), ptr::null_mut());

    let mut n = 0;
    ptr.set(&mut n);
    assert_eq!(ptr.get(), &mut n as *mut i32);
}

#[test]
fn test_tss_ptr_2() {
    use std::ptr;
    use std::thread;

    lazy_static! {
        static ref PTR: TssPtr<i32> = TssPtr::new().unwrap();
    };
    assert_eq!(PTR.get(), ptr::null_mut());

    let mut n = 0;
    PTR.set(&mut n);
    assert_eq!(PTR.get(), &mut n as *mut i32);

    thread::spawn(|| {
        assert_eq!(PTR.get(), ptr::null_mut());

        let mut n = 0;
        PTR.set(&mut n);
        assert_eq!(PTR.get(), &mut n as *mut i32);
    }).join().unwrap();

    thread::spawn(|| {
        assert_eq!(PTR.get(), ptr::null_mut());

        let mut n = 0;
        PTR.set(&mut n);
        assert_eq!(PTR.get(), &mut n as *mut i32);
    }).join().unwrap();
}
