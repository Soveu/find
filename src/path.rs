use std::ffi::CStr;
use std::ops::Deref;
use std::fmt;

/// A wrapper over Vec<u8> that keeps a null byte at the end.
#[derive(Clone)]
#[repr(transparent)]
pub struct Path(Vec<u8>);

impl Deref for Path {
    type Target = CStr;
    fn deref(&self) -> &Self::Target {
        // SAFETY: see Path invariants
        unsafe { CStr::from_bytes_with_nul_unchecked(self.0.as_slice()) }
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl Path {
    /// Constructs a new Path with a nul byte inside
    pub fn from_str(s: &str) -> Self {
        let mut v = Vec::with_capacity(s.len() + 1);
        v.extend_from_slice(s.as_bytes());
        v.push(0u8);
        Self(v)
    }

    #[allow(dead_code)]
    pub unsafe fn from_vec_unchecked(v: Vec<u8>) -> Self {
        Self(v)
    }

    #[allow(dead_code)]
    pub fn from_cstr(s: &CStr) -> Self {
        Self(s.to_bytes_with_nul().to_vec())
    }

    pub fn joined(dir: &CStr, fname: &CStr) -> Self {
        let total = dir.to_bytes().len() + fname.to_bytes().len() + 1;
        let mut v = Vec::with_capacity(total);
        v.extend_from_slice(dir.to_bytes());
        v.extend_from_slice(fname.to_bytes_with_nul());
        Self(v)
    }

    pub fn push(&mut self, s: &CStr) {
        // SAFETY: len() must be at least 1, because of the null byte.
        unsafe { self.0.set_len(self.0.len() - 1); }
        self.0.extend_from_slice(s.to_bytes());
        self.0.extend_from_slice(b"/\0");
    }

    pub fn truncate(&mut self, new_len: usize) {
        self.0.truncate(new_len);
        self.0.push(0u8);
    }

    pub fn as_cstr(&self) -> &CStr {
        self
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}
