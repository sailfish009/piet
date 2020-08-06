//! Implementation of directwrite custom font collections
#![allow(clippy::transmute_ptr_to_ptr, clippy::cmp_null)]

use std::cell::{Cell, RefCell};
use std::convert::TryInto;
use std::rc::Rc;

use winapi::ctypes::c_void;
use winapi::shared::basetsd::{UINT32, UINT64};
use winapi::shared::minwindef::{BOOL, FALSE, TRUE};
use winapi::shared::winerror::{E_INVALIDARG, HRESULT, SUCCEEDED, S_OK};
use winapi::um::dwrite::{
    IDWriteFactory, DWRITE_FONT_FACE_TYPE, DWRITE_FONT_FACE_TYPE_CFF,
    DWRITE_FONT_FACE_TYPE_TRUETYPE, DWRITE_FONT_FACE_TYPE_UNKNOWN, DWRITE_FONT_FILE_TYPE,
    DWRITE_FONT_FILE_TYPE_CFF, DWRITE_FONT_FILE_TYPE_TRUETYPE, DWRITE_FONT_FILE_TYPE_UNKNOWN,
};

use com::interfaces::IUnknown;

//static ENUMERATOR_KEY: &str = "piet's custom font collection key";
// this confuses the type system sometimes, which wants it to be a borrowed array
// when used inline :shrug:
const EMPTY_SLICE: &[u8] = &[];

type FontData = Rc<[u8]>;

/// Fetch a handle to a com interface from a type that implements that interface.
macro_rules! get_interface {
    ($item:expr, $interface:ty) => {{
        let mut ppv: Option<$interface> = None;
        let item = Box::new($item);
        item.add_ref();
        let iid = &<$interface as com::Interface>::IID as *const _;
        let hr = item.query_interface(iid, &mut ppv as *mut _ as *mut *mut c_void);
        item.release();
        Box::into_raw(item);
        if SUCCEEDED(hr) {
            ppv
        } else {
            None
        }
    }};
}

com::interfaces! {
    #[uuid("cca920e4-52f0-492b-bfa8-29c72ee0a468")]
    pub unsafe interface IDWriteFontCollectionLoader: IUnknown {
        fn create_enumerator_from_key(
            &self,
            factory: *mut IDWriteFactory,
            key: *const c_void,
            key_size: UINT32,
            enumerator_out: *mut Option<IDWriteFontFileEnumerator>,
        ) -> HRESULT;
    }

    #[uuid("EFF8970E-C50F-45E0-9284-291CE5A6F771")]
    pub unsafe interface IDWriteFontFileEnumerator: IUnknown {
        fn move_next(&self, has_current: *mut BOOL) -> HRESULT;
        fn get_current_font_file(&self, file: *mut Option<IDWriteFontFile>) -> HRESULT;
    }

    #[uuid("739d886a-cef5-47dc-8769-1a8b41bebbb0")]
    pub unsafe interface IDWriteFontFile: IUnknown {
        fn get_reference_key(
            &self,
            key: *mut *const c_void,
            key_size: *mut UINT32,
        ) -> HRESULT;
        fn get_loader(&self, loader: *mut Option<IDWriteFontFileLoader>) -> HRESULT;
        fn analyze(
            &self,
            is_supported_file_type: *mut BOOL,
            file_type: *mut DWRITE_FONT_FILE_TYPE,
            face_type: *mut DWRITE_FONT_FACE_TYPE,
            number_of_faces: *mut UINT32,
        ) -> HRESULT;
    }

    #[uuid("6d4865fe-0ab8-4d91-8f62-5dd6be34a3e0")]
    pub unsafe interface IDWriteFontFileStream: IUnknown {
        fn read_file_fragment(
            &self,
            start: *mut *const c_void,
            offset: UINT64,
            length: UINT64,
            ctx: *mut *mut c_void,
        ) -> HRESULT;
        fn release_fragment(&self, ctx: *mut c_void);
        fn get_file_size(&self, size: *mut UINT64) -> HRESULT;
        fn get_last_write_time(&self, last_write_time: *mut UINT64) -> HRESULT;
    }

    #[uuid("727cad4e-d6af-4c9e-8a08-d695b11caa49")]
    pub unsafe interface IDWriteFontFileLoader: IUnknown {
        fn create_stream_from_key(
            &self,
            file_key: *const c_void,
            key_size: UINT32,
            stream: *mut Option<IDWriteFontFileStream>,
        ) -> HRESULT;
    }
}

com::class! {
    pub class PietFontCollectionLoader: IDWriteFontCollectionLoader {
        fonts: RefCell<Rc<Vec<FontData>>>,
    }

    impl IDWriteFontCollectionLoader for PietFontCollectionLoader {
        fn create_enumerator_from_key(
            &self,
            _factory: *mut IDWriteFactory,
            _key: *const c_void,
            _key_size: UINT32,
            enumerator_out: *mut Option<IDWriteFontFileEnumerator>,
        ) -> HRESULT {
            //FIXME: does the key matter? or only if we use one loader to load
            //multiple possible collections?
            let files: Rc<_> = self.fonts.borrow().clone();
            let enumerator = PietFontFileEnumerator::new(files, Cell::new(None));
            unsafe { *enumerator_out = get_interface!(enumerator, IDWriteFontFileEnumerator); }
            S_OK
        }
    }
}

impl Default for PietFontCollectionLoader {
    fn default() -> Self {
        PietFontCollectionLoader::new(RefCell::new(Rc::new(Vec::new())))
    }
}

com::class! {
    pub class PietFontFileEnumerator: IDWriteFontFileEnumerator {
        files: Rc<Vec<FontData>>,
        idx: Cell<Option<usize>>,
    }

    impl IDWriteFontFileEnumerator for PietFontFileEnumerator {
        fn move_next(&self, has_current: *mut BOOL) -> HRESULT {
            let next_idx = self.idx.get().map(|n| n + 1).unwrap_or(0);
            let has_item = if next_idx < self.files.len() {
                TRUE
            } else {
                FALSE
            };
            unsafe { *has_current = has_item; }
            self.idx.set(Some(next_idx));
            S_OK
        }

        fn get_current_font_file(&self, file: *mut Option<IDWriteFontFile>) -> HRESULT {
            debug_assert!(self.idx.get().is_some(), "we expect move_next to always be called before this?");

            if let Some(idx) = self.idx.get() {
                let data = self.files.get(idx).cloned().unwrap();
                let font_file = PietFontFile::new(data);
                unsafe { *file = get_interface!(font_file, IDWriteFontFile); }
            }
            S_OK
        }
    }
}

impl Default for PietFontFileEnumerator {
    fn default() -> Self {
        PietFontFileEnumerator::new(Rc::new(Vec::new()), Cell::new(None))
    }
}

com::class! {
    pub class PietFontFile: IDWriteFontFile {
        data: FontData,
    }

    impl IDWriteFontFile for PietFontFile {
        fn get_reference_key(
            &self,
            key: *mut *const c_void,
            key_size: *mut UINT32,
        ) -> HRESULT {
            // we just use our data as our key? You want to talk to the manager?
            let data_len: UINT32 = self.data.len().try_into().unwrap();
            let data_ptr = self.data.as_ptr() as *const c_void;
            unsafe {
                *key = data_ptr;
                *key_size = data_len;
            }
            S_OK
        }

        fn get_loader(&self, loader: *mut Option<IDWriteFontFileLoader>) -> HRESULT {
            let my_loader = PietFontFileLoader::new(self.data.clone());
            unsafe { *loader = get_interface!(my_loader, IDWriteFontFileLoader); }
            S_OK
        }

        fn analyze(
            &self,
            is_supported_file_type: *mut BOOL,
            file_type: *mut DWRITE_FONT_FILE_TYPE,
            face_type: *mut DWRITE_FONT_FACE_TYPE,
            number_of_faces: *mut UINT32,
        ) -> HRESULT {
            let header = [self.data[0], self.data[1], self.data[2], self.data[3]];
            let (this_file_type, this_face_type) = match u32::from_le_bytes(header) {
                // magic numbers from https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6.html#Directory
                0x74727565 | 0x00010000 => (
                    DWRITE_FONT_FILE_TYPE_TRUETYPE,
                    DWRITE_FONT_FACE_TYPE_TRUETYPE,
                ),
                0x4F54544F => (DWRITE_FONT_FILE_TYPE_CFF, DWRITE_FONT_FACE_TYPE_CFF),
                _ => (DWRITE_FONT_FILE_TYPE_UNKNOWN, DWRITE_FONT_FACE_TYPE_UNKNOWN),
            };
            let supported = if this_file_type != DWRITE_FONT_FILE_TYPE_UNKNOWN {
                TRUE
            } else {
                FALSE
            };
            unsafe {
                *is_supported_file_type = supported;
                *file_type = this_file_type;
                *face_type = this_face_type;
                // could be 0 if unsupported? seems unlikely to matter.
                *number_of_faces = 1;
            }
            S_OK
        }
    }
}

impl Default for PietFontFile {
    fn default() -> Self {
        PietFontFile::new(Rc::from(EMPTY_SLICE))
    }
}

com::class! {
    pub class PietFontFileLoader: IDWriteFontFileLoader {
        data: FontData,
    }

    impl IDWriteFontFileLoader for PietFontFileLoader {
        fn create_stream_from_key(
            &self,
            _file_key: *const c_void,
            key_size: UINT32,
            stream: *mut Option<IDWriteFontFileStream>,
        ) -> HRESULT {
            if key_size as usize != self.data.len() {
                return E_INVALIDARG;
            }
            let my_stream = PietFontFileStream::new(self.data.clone());
            unsafe { *stream = get_interface!(my_stream, IDWriteFontFileStream); }
            S_OK
        }
    }
}

impl Default for PietFontFileLoader {
    fn default() -> Self {
        PietFontFileLoader::new(Rc::from(EMPTY_SLICE))
    }
}

com::class! {
    pub class PietFontFileStream: IDWriteFontFileStream {
        data: FontData,
    }

    impl IDWriteFontFileStream for PietFontFileStream {
        fn read_file_fragment(
            &self,
            start: *mut *const c_void,
            offset: UINT64,
            length: UINT64,
            ctx: *mut *mut c_void,
        ) -> HRESULT {
            if offset + length > self.data.len() as UINT64 {
                return E_INVALIDARG;
            }

            unsafe {
                let ptr = self.data.as_ptr().add(offset as usize);
                *start = ptr as *const c_void;
                *ctx = std::ptr::null_mut();
            }
            S_OK
        }

        fn release_fragment(&self, _ctx: *mut c_void) {}

        fn get_file_size(&self, size: *mut UINT64) -> HRESULT {

            unsafe { *size = self.data.len() as UINT64; }
            S_OK
        }

        fn get_last_write_time(&self, last_write_time: *mut UINT64) -> HRESULT {
            // FIXME: we could make this meaningful by storing a timestamp when the font is
            // added in piet? I can't think of anything this would buy us, though.

            // arbitrary small value that isn't 0, because I don't trust 0
            unsafe { *last_write_time = 10; }
            S_OK
        }
    }
}

impl Default for PietFontFileStream {
    fn default() -> Self {
        PietFontFileStream::new(Rc::from(EMPTY_SLICE))
    }
}
