mod error;

use std::ffi::{CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;
use cxx;
use itertools::Itertools;
use crate::error::{ErrorKind, Error, Context};
use crate::ErrorKind::{IOError, ValueError};


#[cxx::bridge(namespace = "Darts")]
pub(crate) mod ffi {

    // C++ types and signatures exposed to Rust.
    unsafe extern "C++" {
        include!("darts.h");
        include!("bridge.h");

        pub(crate) type DoubleArray;

        #[namespace = "bridge"]
        pub(crate) unsafe fn new_datrie() -> UniquePtr<DoubleArray>;

        // #[rust_name = "exact_match_search"]
        #[cxx_name = "exactMatchSearch"]
        pub(crate) unsafe fn exact_match_search(
            &self, key: *const c_char, result: & mut i32, length: usize, node_pos: usize);

        #[cxx_name = "commonPrefixSearch"]
        pub(crate) unsafe fn common_prefix_search(
            &self, key: *const c_char, result: *mut i32,
            max_num_results: usize, length: usize, node_pos: usize) -> usize;

        pub(crate) unsafe fn traverse(&self, key: *const c_char, node_pos: &mut usize,
                                      key_pos: &mut usize, length: usize) -> i32;

        // 由于目前 cxx 不支持 void* 类型 (c_void), 不得不自己对获取 array 和 设置 array 两个函数进行再次封装
        #[namespace = "bridge"]
        pub(crate) unsafe fn set_array(da: Pin<&mut DoubleArray>, ptr: *const u32, size: usize);

        #[namespace = "bridge"]
        pub(crate) unsafe fn get_array(da: &DoubleArray) -> *const u32;

        pub(crate) unsafe fn clear(self: Pin<&mut DoubleArray>);
        pub(crate) fn unit_size(&self) -> usize;
        pub(crate) fn size(&self) -> usize;
        pub(crate) fn total_size(&self) -> usize;

        // 由于 cxx 会自动将 fn 映射为其自己实现的 Fn c++ 类, 无法映射为 C 的函数指针.
        // 目前没有找到更好的方法来处理函数指针, 导致无法直接映射到 darts-clone 中定义的 build 成员函数.
        // unsafe fn build(self: Pin<&mut DoubleArray>, num_keys: usize, keys: *const *const c_char,
        //                 lengths: *const usize,
        //                 values: *const i32,
        //                 progress_func: fn(usize, usize) -> i32) -> i32;
        #[namespace = "bridge"]
        pub(crate) unsafe fn build(da: Pin<&mut DoubleArray>,
                                   num_keys: usize, keys: *const *const c_char,
                                   lengths: *const usize,
                                   values: *const i32) -> i32;

        #[cxx_name = "open"]
        pub(crate) unsafe fn load(self: Pin<&mut Self>,
                       filename: *const c_char,
                       mode: *const c_char,
                       offset: usize,
                       size: usize) -> i32;
        #[cxx_name = "save"]
        pub(crate) unsafe fn dump(&self, filename: *const c_char, mode: *const c_char, offset: usize) -> i32;
    }
}

pub struct Datrie {
    intern: cxx::UniquePtr<ffi::DoubleArray>,
    array_buf: Option<Vec<u32>>,
}

impl Datrie {

    pub fn new() -> Self {
        unsafe { Datrie { intern: ffi::new_datrie(), array_buf: None } }
    }

    pub fn find(&self, key: &str, node_pos: Option<usize>) -> Option<i32> {
        if self.is_empty() {
            return None
        }

        unsafe {
            let mut result: i32 = -1;
            self.intern.exact_match_search(
                key.as_ptr() as *const _, &mut result, key.len(), node_pos.unwrap_or(0));
            return if result >= 0 { Some(result) } else { None };
        }
    }

    pub fn is_empty(&self) -> bool {
        unsafe {
            ffi::get_array(&*self.intern).is_null() || self.size() == 0
        }
    }

    pub fn common_prefix_search(&self, key: &str, num_result: usize, node_pos: Option<usize>) -> (Vec<i32>, usize) {
        let mut results = Vec::new();

        if self.is_empty() {
            return (results, 0);
        }

        results.resize(num_result, -1);
        let num = unsafe {
            self.intern.common_prefix_search(
                key.as_ptr() as *const _, results.as_mut_ptr(), num_result,
                key.len(), node_pos.unwrap_or(0))
        };

        if num < num_result {
            results.resize(num, -1);
        }
        return (results, num);
    }

    pub fn traverse(&self, key: &str, node_pos: &mut usize, key_pos: &mut usize) -> i32 {
        if self.is_empty() {
            return -2;
        }
        unsafe {
            self.intern.traverse(key.as_ptr() as *const _, node_pos, key_pos, key.len())
        }
    }

    pub fn build(&mut self, keys: &[&str], values: Option<&[i32]>) -> Result<(), Error> {
        let num_keys = keys.len();
        if num_keys == 0 {
            return Err(ErrorKind::ValueError("empty keys".into()).into());
        }

        let sorted_keys: Vec<&str>;
        let sorted_values: Vec<i32>;
        let c_values;

        if let Some(values) = values {
            if num_keys != values.len() {
                return Err(ErrorKind::ValueError(
                    format!("number of values and values mismatch: got {num_keys} keys and {num_values} values",
                        num_keys = num_keys, num_values = values.len()).into()).into());
            }
            let (mut keys, mut values): (Vec<&str>, Vec<i32>) = keys
                .into_iter()
                .zip(values)
                .sorted_by(|a, b| Ord::cmp(a.0, b.0))
                .unzip();
            sorted_keys = std::mem::take(&mut keys);
            sorted_values = std::mem::take(&mut values);
            c_values = sorted_values.as_ptr();
        } else {
            sorted_keys = keys.into_iter()
                .sorted()
                .map(|x| *x)
                .collect::<Vec<_>>();
            c_values = ptr::null();
        }

        let (sorted_keys, lengths): (Vec<*const c_char>, Vec<usize>) = sorted_keys
            .into_iter()
            .map(|x| (x.as_ptr() as *const c_char, x.len()))
            .unzip();
        let c_keys = sorted_keys.as_ptr();
        let lengths = lengths.as_ptr();

        // 要特别注意使用裸指针时, 其指向的内存的有效性. 不能指向生命周期结束的对象的内存.
        // 使用裸指针时, borrow checker 并不能进行检查.
        let status = unsafe {
            ffi::build(self.intern.pin_mut(), num_keys, c_keys, lengths, c_values)
        };

        if status != 0 {
            Err(ErrorKind::UnknownError("build double array trie failed".into()).into())
        } else {
            Ok(())
        }
    }

    pub fn clear(&mut self) {
        unsafe { self.intern.pin_mut().clear(); }
        self.array_buf = None;
    }

    pub fn unit_size(&self) -> usize {
        self.intern.unit_size()
    }

    pub fn size(&self) -> usize {
        self.intern.size()
    }

    pub fn total_size(&self) -> usize {
        self.intern.total_size()
    }

    pub fn set_array(&mut self, array: Vec<u32>) {
        unsafe {
            ffi::set_array(self.intern.pin_mut(), array.as_ptr(), array.len());
        }
        self.array_buf = Some(array);
    }

    pub fn get_array(&mut self) -> Option<&Vec<u32>> {
        if self.array_buf.is_some() {
            return self.array_buf.as_ref();
        }

        let p = unsafe {
            ffi::get_array(&*self.intern)
        };

        if p.is_null() {
            None
        } else {
            let siz = self.size();
            let arr: Vec<_> = (0..siz)
                .map(|i| unsafe { *p.offset(i as isize) })
                .collect();
            self.array_buf = Some(arr);
            self.array_buf.as_ref()
        }
    }

    pub fn load<P: AsRef<Path>>(&mut self, filename: P, offset: Option<usize>, size: Option<usize>)
                                -> Result<(), Error> {

        let c_filename = filename.as_ref()
            .to_str()
            .ok_or_else(|| Error::from(ValueError("can not convert filename to string".into())))?;
        let c_filename = CString::new(c_filename).or_else(
            |_| Err(Error::from(ValueError("NUL in filename string".into())))
        )?;

        const MODE: &str = "rb";

        let status = unsafe {
            self.intern.pin_mut().load(c_filename.as_ptr(), MODE.as_ptr() as *const _,
                                       offset.unwrap_or(0usize), size.unwrap_or(0usize))
        };

        if status != 0 {
            let err: Error = IOError(std::io::Error::from(std::io::ErrorKind::Other)).into();
            return Err(err).with_context(||
                format!("failed to load double array trie from '{:?}'",
                        filename.as_ref().as_os_str()).into());
        }

        Ok(())
    }

    pub fn dump<P: AsRef<Path>>(&self, filename: P, mode: Option<&str>, offset: Option<usize>)
                                -> Result<(), Error> {
        let mode = mode.unwrap_or("wb");
        let offset = offset.unwrap_or(0usize);

        let c_filename = filename.as_ref()
            .to_str()
            .ok_or_else(|| Error::from(ValueError("can not convert filename to string".into())))?;
        let c_filename = CString::new(c_filename).or_else(
            |_| Err(Error::from(ValueError("NUL in filename string".into())))
        )?;

        let status = unsafe {
            self.intern.dump(c_filename.as_ptr(), mode.as_ptr() as *const _, offset)
        };

        if status != 0 {
            let err: Error = IOError(std::io::Error::from(std::io::ErrorKind::Other)).into();
            return Err(err).with_context(||
                format!("failed to save double array trie to '{:?}'",
                        filename.as_ref().as_os_str()).into());
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let da = Datrie::new();
        assert!(da.find("hello", None).is_none());
        assert!(da.is_empty());
    }

    #[test]
    fn test_builder() {
        let mut da = Datrie::new();

        let keys = &["hello", "world", "he", "hell"];
        let values = &[0, 1, 2, 3];
        da.build(keys, Some(values))
            .expect("build failed");

        for (key, value) in keys.iter().zip(values) {
            assert_eq!(da.find(key, None).unwrap_or(-1), *value);
        }

        da.clear();
        assert!(da.is_empty());
    }

    #[test]
    fn test_common_prefix_search() {
        let mut da = Datrie::new();
        let keys = &["hello", "world", "he", "hell"];
        let values = &[0, 1, 2, 3];
        da.build(keys, Some(values))
            .expect("build failed");

        let (v, s) = da.common_prefix_search("hello", 2, None);
        assert_eq!(v, vec![2, 3]);
        assert_eq!(s, 3);
    }

    #[test]
    fn test_traverse() {
        let mut da = Datrie::new();
        let keys = &["hello", "world", "he"];
        let values = &[0, 1, 2];
        da.build(keys, Some(values))
            .expect("build failed");

        let mut key_pos = 0usize;
        let mut node_pos = 0usize;
        let s = da.traverse("hel", &mut node_pos, &mut key_pos);
        assert_eq!(s, -1);
        key_pos = 0;
        let s = da.traverse("l", &mut node_pos, &mut key_pos);
        assert_eq!(s, -1);
        key_pos = 0;
        let s = da.traverse("o", &mut node_pos, &mut key_pos);
        assert!(s >= 0);
    }
}