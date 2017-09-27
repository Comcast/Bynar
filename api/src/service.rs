// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

use protobuf::Message as Message_imported_for_functions;
use protobuf::ProtobufEnum as ProtobufEnum_imported_for_functions;

#[derive(PartialEq,Clone,Default)]
pub struct Osd {
    // message fields
    fsid: ::protobuf::SingularField<::std::string::String>,
    id: ::std::option::Option<u64>,
    block_device: ::protobuf::SingularField<::std::string::String>,
    active: ::std::option::Option<bool>,
    used_space: ::std::option::Option<u64>,
    total_space: ::std::option::Option<u64>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for Osd {}

impl Osd {
    pub fn new() -> Osd {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static Osd {
        static mut instance: ::protobuf::lazy::Lazy<Osd> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const Osd,
        };
        unsafe {
            instance.get(Osd::new)
        }
    }

    // optional string fsid = 1;

    pub fn clear_fsid(&mut self) {
        self.fsid.clear();
    }

    pub fn has_fsid(&self) -> bool {
        self.fsid.is_some()
    }

    // Param is passed by value, moved
    pub fn set_fsid(&mut self, v: ::std::string::String) {
        self.fsid = ::protobuf::SingularField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_fsid(&mut self) -> &mut ::std::string::String {
        if self.fsid.is_none() {
            self.fsid.set_default();
        }
        self.fsid.as_mut().unwrap()
    }

    // Take field
    pub fn take_fsid(&mut self) -> ::std::string::String {
        self.fsid.take().unwrap_or_else(|| ::std::string::String::new())
    }

    pub fn get_fsid(&self) -> &str {
        match self.fsid.as_ref() {
            Some(v) => &v,
            None => "",
        }
    }

    fn get_fsid_for_reflect(&self) -> &::protobuf::SingularField<::std::string::String> {
        &self.fsid
    }

    fn mut_fsid_for_reflect(&mut self) -> &mut ::protobuf::SingularField<::std::string::String> {
        &mut self.fsid
    }

    // required uint64 id = 2;

    pub fn clear_id(&mut self) {
        self.id = ::std::option::Option::None;
    }

    pub fn has_id(&self) -> bool {
        self.id.is_some()
    }

    // Param is passed by value, moved
    pub fn set_id(&mut self, v: u64) {
        self.id = ::std::option::Option::Some(v);
    }

    pub fn get_id(&self) -> u64 {
        self.id.unwrap_or(0)
    }

    fn get_id_for_reflect(&self) -> &::std::option::Option<u64> {
        &self.id
    }

    fn mut_id_for_reflect(&mut self) -> &mut ::std::option::Option<u64> {
        &mut self.id
    }

    // optional string block_device = 3;

    pub fn clear_block_device(&mut self) {
        self.block_device.clear();
    }

    pub fn has_block_device(&self) -> bool {
        self.block_device.is_some()
    }

    // Param is passed by value, moved
    pub fn set_block_device(&mut self, v: ::std::string::String) {
        self.block_device = ::protobuf::SingularField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_block_device(&mut self) -> &mut ::std::string::String {
        if self.block_device.is_none() {
            self.block_device.set_default();
        }
        self.block_device.as_mut().unwrap()
    }

    // Take field
    pub fn take_block_device(&mut self) -> ::std::string::String {
        self.block_device.take().unwrap_or_else(|| ::std::string::String::new())
    }

    pub fn get_block_device(&self) -> &str {
        match self.block_device.as_ref() {
            Some(v) => &v,
            None => "",
        }
    }

    fn get_block_device_for_reflect(&self) -> &::protobuf::SingularField<::std::string::String> {
        &self.block_device
    }

    fn mut_block_device_for_reflect(&mut self) -> &mut ::protobuf::SingularField<::std::string::String> {
        &mut self.block_device
    }

    // required bool active = 4;

    pub fn clear_active(&mut self) {
        self.active = ::std::option::Option::None;
    }

    pub fn has_active(&self) -> bool {
        self.active.is_some()
    }

    // Param is passed by value, moved
    pub fn set_active(&mut self, v: bool) {
        self.active = ::std::option::Option::Some(v);
    }

    pub fn get_active(&self) -> bool {
        self.active.unwrap_or(false)
    }

    fn get_active_for_reflect(&self) -> &::std::option::Option<bool> {
        &self.active
    }

    fn mut_active_for_reflect(&mut self) -> &mut ::std::option::Option<bool> {
        &mut self.active
    }

    // required uint64 used_space = 5;

    pub fn clear_used_space(&mut self) {
        self.used_space = ::std::option::Option::None;
    }

    pub fn has_used_space(&self) -> bool {
        self.used_space.is_some()
    }

    // Param is passed by value, moved
    pub fn set_used_space(&mut self, v: u64) {
        self.used_space = ::std::option::Option::Some(v);
    }

    pub fn get_used_space(&self) -> u64 {
        self.used_space.unwrap_or(0)
    }

    fn get_used_space_for_reflect(&self) -> &::std::option::Option<u64> {
        &self.used_space
    }

    fn mut_used_space_for_reflect(&mut self) -> &mut ::std::option::Option<u64> {
        &mut self.used_space
    }

    // required uint64 total_space = 6;

    pub fn clear_total_space(&mut self) {
        self.total_space = ::std::option::Option::None;
    }

    pub fn has_total_space(&self) -> bool {
        self.total_space.is_some()
    }

    // Param is passed by value, moved
    pub fn set_total_space(&mut self, v: u64) {
        self.total_space = ::std::option::Option::Some(v);
    }

    pub fn get_total_space(&self) -> u64 {
        self.total_space.unwrap_or(0)
    }

    fn get_total_space_for_reflect(&self) -> &::std::option::Option<u64> {
        &self.total_space
    }

    fn mut_total_space_for_reflect(&mut self) -> &mut ::std::option::Option<u64> {
        &mut self.total_space
    }
}

impl ::protobuf::Message for Osd {
    fn is_initialized(&self) -> bool {
        if self.id.is_none() {
            return false;
        }
        if self.active.is_none() {
            return false;
        }
        if self.used_space.is_none() {
            return false;
        }
        if self.total_space.is_none() {
            return false;
        }
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_singular_string_into(wire_type, is, &mut self.fsid)?;
                },
                2 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.id = ::std::option::Option::Some(tmp);
                },
                3 => {
                    ::protobuf::rt::read_singular_string_into(wire_type, is, &mut self.block_device)?;
                },
                4 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_bool()?;
                    self.active = ::std::option::Option::Some(tmp);
                },
                5 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.used_space = ::std::option::Option::Some(tmp);
                },
                6 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_uint64()?;
                    self.total_space = ::std::option::Option::Some(tmp);
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if let Some(ref v) = self.fsid.as_ref() {
            my_size += ::protobuf::rt::string_size(1, &v);
        }
        if let Some(v) = self.id {
            my_size += ::protobuf::rt::value_size(2, v, ::protobuf::wire_format::WireTypeVarint);
        }
        if let Some(ref v) = self.block_device.as_ref() {
            my_size += ::protobuf::rt::string_size(3, &v);
        }
        if let Some(v) = self.active {
            my_size += 2;
        }
        if let Some(v) = self.used_space {
            my_size += ::protobuf::rt::value_size(5, v, ::protobuf::wire_format::WireTypeVarint);
        }
        if let Some(v) = self.total_space {
            my_size += ::protobuf::rt::value_size(6, v, ::protobuf::wire_format::WireTypeVarint);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if let Some(ref v) = self.fsid.as_ref() {
            os.write_string(1, &v)?;
        }
        if let Some(v) = self.id {
            os.write_uint64(2, v)?;
        }
        if let Some(ref v) = self.block_device.as_ref() {
            os.write_string(3, &v)?;
        }
        if let Some(v) = self.active {
            os.write_bool(4, v)?;
        }
        if let Some(v) = self.used_space {
            os.write_uint64(5, v)?;
        }
        if let Some(v) = self.total_space {
            os.write_uint64(6, v)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &::std::any::Any {
        self as &::std::any::Any
    }
    fn as_any_mut(&mut self) -> &mut ::std::any::Any {
        self as &mut ::std::any::Any
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<::std::any::Any> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for Osd {
    fn new() -> Osd {
        Osd::new()
    }

    fn descriptor_static(_: ::std::option::Option<Osd>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_singular_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "fsid",
                    Osd::get_fsid_for_reflect,
                    Osd::mut_fsid_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "id",
                    Osd::get_id_for_reflect,
                    Osd::mut_id_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_singular_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "block_device",
                    Osd::get_block_device_for_reflect,
                    Osd::mut_block_device_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeBool>(
                    "active",
                    Osd::get_active_for_reflect,
                    Osd::mut_active_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "used_space",
                    Osd::get_used_space_for_reflect,
                    Osd::mut_used_space_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeUint64>(
                    "total_space",
                    Osd::get_total_space_for_reflect,
                    Osd::mut_total_space_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<Osd>(
                    "Osd",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for Osd {
    fn clear(&mut self) {
        self.clear_fsid();
        self.clear_id();
        self.clear_block_device();
        self.clear_active();
        self.clear_used_space();
        self.clear_total_space();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Osd {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Osd {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct Disk {
    // message fields
    field_type: ::std::option::Option<DiskType>,
    dev_path: ::protobuf::SingularField<::std::string::String>,
    serial_number: ::protobuf::SingularField<::std::string::String>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for Disk {}

impl Disk {
    pub fn new() -> Disk {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static Disk {
        static mut instance: ::protobuf::lazy::Lazy<Disk> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const Disk,
        };
        unsafe {
            instance.get(Disk::new)
        }
    }

    // required .ceph_disk.DiskType type = 1;

    pub fn clear_field_type(&mut self) {
        self.field_type = ::std::option::Option::None;
    }

    pub fn has_field_type(&self) -> bool {
        self.field_type.is_some()
    }

    // Param is passed by value, moved
    pub fn set_field_type(&mut self, v: DiskType) {
        self.field_type = ::std::option::Option::Some(v);
    }

    pub fn get_field_type(&self) -> DiskType {
        self.field_type.unwrap_or(DiskType::SOLID_STATE)
    }

    fn get_field_type_for_reflect(&self) -> &::std::option::Option<DiskType> {
        &self.field_type
    }

    fn mut_field_type_for_reflect(&mut self) -> &mut ::std::option::Option<DiskType> {
        &mut self.field_type
    }

    // required string dev_path = 2;

    pub fn clear_dev_path(&mut self) {
        self.dev_path.clear();
    }

    pub fn has_dev_path(&self) -> bool {
        self.dev_path.is_some()
    }

    // Param is passed by value, moved
    pub fn set_dev_path(&mut self, v: ::std::string::String) {
        self.dev_path = ::protobuf::SingularField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_dev_path(&mut self) -> &mut ::std::string::String {
        if self.dev_path.is_none() {
            self.dev_path.set_default();
        }
        self.dev_path.as_mut().unwrap()
    }

    // Take field
    pub fn take_dev_path(&mut self) -> ::std::string::String {
        self.dev_path.take().unwrap_or_else(|| ::std::string::String::new())
    }

    pub fn get_dev_path(&self) -> &str {
        match self.dev_path.as_ref() {
            Some(v) => &v,
            None => "",
        }
    }

    fn get_dev_path_for_reflect(&self) -> &::protobuf::SingularField<::std::string::String> {
        &self.dev_path
    }

    fn mut_dev_path_for_reflect(&mut self) -> &mut ::protobuf::SingularField<::std::string::String> {
        &mut self.dev_path
    }

    // optional string serial_number = 3;

    pub fn clear_serial_number(&mut self) {
        self.serial_number.clear();
    }

    pub fn has_serial_number(&self) -> bool {
        self.serial_number.is_some()
    }

    // Param is passed by value, moved
    pub fn set_serial_number(&mut self, v: ::std::string::String) {
        self.serial_number = ::protobuf::SingularField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_serial_number(&mut self) -> &mut ::std::string::String {
        if self.serial_number.is_none() {
            self.serial_number.set_default();
        }
        self.serial_number.as_mut().unwrap()
    }

    // Take field
    pub fn take_serial_number(&mut self) -> ::std::string::String {
        self.serial_number.take().unwrap_or_else(|| ::std::string::String::new())
    }

    pub fn get_serial_number(&self) -> &str {
        match self.serial_number.as_ref() {
            Some(v) => &v,
            None => "",
        }
    }

    fn get_serial_number_for_reflect(&self) -> &::protobuf::SingularField<::std::string::String> {
        &self.serial_number
    }

    fn mut_serial_number_for_reflect(&mut self) -> &mut ::protobuf::SingularField<::std::string::String> {
        &mut self.serial_number
    }
}

impl ::protobuf::Message for Disk {
    fn is_initialized(&self) -> bool {
        if self.field_type.is_none() {
            return false;
        }
        if self.dev_path.is_none() {
            return false;
        }
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_enum()?;
                    self.field_type = ::std::option::Option::Some(tmp);
                },
                2 => {
                    ::protobuf::rt::read_singular_string_into(wire_type, is, &mut self.dev_path)?;
                },
                3 => {
                    ::protobuf::rt::read_singular_string_into(wire_type, is, &mut self.serial_number)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if let Some(v) = self.field_type {
            my_size += ::protobuf::rt::enum_size(1, v);
        }
        if let Some(ref v) = self.dev_path.as_ref() {
            my_size += ::protobuf::rt::string_size(2, &v);
        }
        if let Some(ref v) = self.serial_number.as_ref() {
            my_size += ::protobuf::rt::string_size(3, &v);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if let Some(v) = self.field_type {
            os.write_enum(1, v.value())?;
        }
        if let Some(ref v) = self.dev_path.as_ref() {
            os.write_string(2, &v)?;
        }
        if let Some(ref v) = self.serial_number.as_ref() {
            os.write_string(3, &v)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &::std::any::Any {
        self as &::std::any::Any
    }
    fn as_any_mut(&mut self) -> &mut ::std::any::Any {
        self as &mut ::std::any::Any
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<::std::any::Any> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for Disk {
    fn new() -> Disk {
        Disk::new()
    }

    fn descriptor_static(_: ::std::option::Option<Disk>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeEnum<DiskType>>(
                    "type",
                    Disk::get_field_type_for_reflect,
                    Disk::mut_field_type_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_singular_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "dev_path",
                    Disk::get_dev_path_for_reflect,
                    Disk::mut_dev_path_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_singular_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "serial_number",
                    Disk::get_serial_number_for_reflect,
                    Disk::mut_serial_number_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<Disk>(
                    "Disk",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for Disk {
    fn clear(&mut self) {
        self.clear_field_type();
        self.clear_dev_path();
        self.clear_serial_number();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Disk {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Disk {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct Disks {
    // message fields
    disk: ::protobuf::RepeatedField<Disk>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for Disks {}

impl Disks {
    pub fn new() -> Disks {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static Disks {
        static mut instance: ::protobuf::lazy::Lazy<Disks> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const Disks,
        };
        unsafe {
            instance.get(Disks::new)
        }
    }

    // repeated .ceph_disk.Disk disk = 1;

    pub fn clear_disk(&mut self) {
        self.disk.clear();
    }

    // Param is passed by value, moved
    pub fn set_disk(&mut self, v: ::protobuf::RepeatedField<Disk>) {
        self.disk = v;
    }

    // Mutable pointer to the field.
    pub fn mut_disk(&mut self) -> &mut ::protobuf::RepeatedField<Disk> {
        &mut self.disk
    }

    // Take field
    pub fn take_disk(&mut self) -> ::protobuf::RepeatedField<Disk> {
        ::std::mem::replace(&mut self.disk, ::protobuf::RepeatedField::new())
    }

    pub fn get_disk(&self) -> &[Disk] {
        &self.disk
    }

    fn get_disk_for_reflect(&self) -> &::protobuf::RepeatedField<Disk> {
        &self.disk
    }

    fn mut_disk_for_reflect(&mut self) -> &mut ::protobuf::RepeatedField<Disk> {
        &mut self.disk
    }
}

impl ::protobuf::Message for Disks {
    fn is_initialized(&self) -> bool {
        for v in &self.disk {
            if !v.is_initialized() {
                return false;
            }
        };
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    ::protobuf::rt::read_repeated_message_into(wire_type, is, &mut self.disk)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        for value in &self.disk {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint32_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        for v in &self.disk {
            os.write_tag(1, ::protobuf::wire_format::WireTypeLengthDelimited)?;
            os.write_raw_varint32(v.get_cached_size())?;
            v.write_to_with_cached_sizes(os)?;
        };
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &::std::any::Any {
        self as &::std::any::Any
    }
    fn as_any_mut(&mut self) -> &mut ::std::any::Any {
        self as &mut ::std::any::Any
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<::std::any::Any> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for Disks {
    fn new() -> Disks {
        Disks::new()
    }

    fn descriptor_static(_: ::std::option::Option<Disks>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_repeated_field_accessor::<_, ::protobuf::types::ProtobufTypeMessage<Disk>>(
                    "disk",
                    Disks::get_disk_for_reflect,
                    Disks::mut_disk_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<Disks>(
                    "Disks",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for Disks {
    fn clear(&mut self) {
        self.clear_disk();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Disks {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Disks {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct OpResult {
    // message fields
    result: ::std::option::Option<OpResult_ResultType>,
    error_msg: ::protobuf::SingularField<::std::string::String>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for OpResult {}

impl OpResult {
    pub fn new() -> OpResult {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static OpResult {
        static mut instance: ::protobuf::lazy::Lazy<OpResult> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const OpResult,
        };
        unsafe {
            instance.get(OpResult::new)
        }
    }

    // required .ceph_disk.OpResult.ResultType result = 1;

    pub fn clear_result(&mut self) {
        self.result = ::std::option::Option::None;
    }

    pub fn has_result(&self) -> bool {
        self.result.is_some()
    }

    // Param is passed by value, moved
    pub fn set_result(&mut self, v: OpResult_ResultType) {
        self.result = ::std::option::Option::Some(v);
    }

    pub fn get_result(&self) -> OpResult_ResultType {
        self.result.unwrap_or(OpResult_ResultType::OK)
    }

    fn get_result_for_reflect(&self) -> &::std::option::Option<OpResult_ResultType> {
        &self.result
    }

    fn mut_result_for_reflect(&mut self) -> &mut ::std::option::Option<OpResult_ResultType> {
        &mut self.result
    }

    // optional string error_msg = 2;

    pub fn clear_error_msg(&mut self) {
        self.error_msg.clear();
    }

    pub fn has_error_msg(&self) -> bool {
        self.error_msg.is_some()
    }

    // Param is passed by value, moved
    pub fn set_error_msg(&mut self, v: ::std::string::String) {
        self.error_msg = ::protobuf::SingularField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_error_msg(&mut self) -> &mut ::std::string::String {
        if self.error_msg.is_none() {
            self.error_msg.set_default();
        }
        self.error_msg.as_mut().unwrap()
    }

    // Take field
    pub fn take_error_msg(&mut self) -> ::std::string::String {
        self.error_msg.take().unwrap_or_else(|| ::std::string::String::new())
    }

    pub fn get_error_msg(&self) -> &str {
        match self.error_msg.as_ref() {
            Some(v) => &v,
            None => "",
        }
    }

    fn get_error_msg_for_reflect(&self) -> &::protobuf::SingularField<::std::string::String> {
        &self.error_msg
    }

    fn mut_error_msg_for_reflect(&mut self) -> &mut ::protobuf::SingularField<::std::string::String> {
        &mut self.error_msg
    }
}

impl ::protobuf::Message for OpResult {
    fn is_initialized(&self) -> bool {
        if self.result.is_none() {
            return false;
        }
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_enum()?;
                    self.result = ::std::option::Option::Some(tmp);
                },
                2 => {
                    ::protobuf::rt::read_singular_string_into(wire_type, is, &mut self.error_msg)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if let Some(v) = self.result {
            my_size += ::protobuf::rt::enum_size(1, v);
        }
        if let Some(ref v) = self.error_msg.as_ref() {
            my_size += ::protobuf::rt::string_size(2, &v);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if let Some(v) = self.result {
            os.write_enum(1, v.value())?;
        }
        if let Some(ref v) = self.error_msg.as_ref() {
            os.write_string(2, &v)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &::std::any::Any {
        self as &::std::any::Any
    }
    fn as_any_mut(&mut self) -> &mut ::std::any::Any {
        self as &mut ::std::any::Any
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<::std::any::Any> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for OpResult {
    fn new() -> OpResult {
        OpResult::new()
    }

    fn descriptor_static(_: ::std::option::Option<OpResult>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeEnum<OpResult_ResultType>>(
                    "result",
                    OpResult::get_result_for_reflect,
                    OpResult::mut_result_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_singular_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "error_msg",
                    OpResult::get_error_msg_for_reflect,
                    OpResult::mut_error_msg_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<OpResult>(
                    "OpResult",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for OpResult {
    fn clear(&mut self) {
        self.clear_result();
        self.clear_error_msg();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for OpResult {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for OpResult {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
pub enum OpResult_ResultType {
    OK = 0,
    ERR = 1,
}

impl ::protobuf::ProtobufEnum for OpResult_ResultType {
    fn value(&self) -> i32 {
        *self as i32
    }

    fn from_i32(value: i32) -> ::std::option::Option<OpResult_ResultType> {
        match value {
            0 => ::std::option::Option::Some(OpResult_ResultType::OK),
            1 => ::std::option::Option::Some(OpResult_ResultType::ERR),
            _ => ::std::option::Option::None
        }
    }

    fn values() -> &'static [Self] {
        static values: &'static [OpResult_ResultType] = &[
            OpResult_ResultType::OK,
            OpResult_ResultType::ERR,
        ];
        values
    }

    fn enum_descriptor_static(_: ::std::option::Option<OpResult_ResultType>) -> &'static ::protobuf::reflect::EnumDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::EnumDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::EnumDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                ::protobuf::reflect::EnumDescriptor::new("OpResult_ResultType", file_descriptor_proto())
            })
        }
    }
}

impl ::std::marker::Copy for OpResult_ResultType {
}

impl ::protobuf::reflect::ProtobufValue for OpResult_ResultType {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Enum(self.descriptor())
    }
}

#[derive(PartialEq,Clone,Default)]
pub struct Operation {
    // message fields
    Op_type: ::std::option::Option<Op>,
    disk: ::protobuf::SingularField<::std::string::String>,
    // special fields
    unknown_fields: ::protobuf::UnknownFields,
    cached_size: ::protobuf::CachedSize,
}

// see codegen.rs for the explanation why impl Sync explicitly
unsafe impl ::std::marker::Sync for Operation {}

impl Operation {
    pub fn new() -> Operation {
        ::std::default::Default::default()
    }

    pub fn default_instance() -> &'static Operation {
        static mut instance: ::protobuf::lazy::Lazy<Operation> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const Operation,
        };
        unsafe {
            instance.get(Operation::new)
        }
    }

    // required .ceph_disk.Op Op_type = 1;

    pub fn clear_Op_type(&mut self) {
        self.Op_type = ::std::option::Option::None;
    }

    pub fn has_Op_type(&self) -> bool {
        self.Op_type.is_some()
    }

    // Param is passed by value, moved
    pub fn set_Op_type(&mut self, v: Op) {
        self.Op_type = ::std::option::Option::Some(v);
    }

    pub fn get_Op_type(&self) -> Op {
        self.Op_type.unwrap_or(Op::Add)
    }

    fn get_Op_type_for_reflect(&self) -> &::std::option::Option<Op> {
        &self.Op_type
    }

    fn mut_Op_type_for_reflect(&mut self) -> &mut ::std::option::Option<Op> {
        &mut self.Op_type
    }

    // optional string disk = 2;

    pub fn clear_disk(&mut self) {
        self.disk.clear();
    }

    pub fn has_disk(&self) -> bool {
        self.disk.is_some()
    }

    // Param is passed by value, moved
    pub fn set_disk(&mut self, v: ::std::string::String) {
        self.disk = ::protobuf::SingularField::some(v);
    }

    // Mutable pointer to the field.
    // If field is not initialized, it is initialized with default value first.
    pub fn mut_disk(&mut self) -> &mut ::std::string::String {
        if self.disk.is_none() {
            self.disk.set_default();
        }
        self.disk.as_mut().unwrap()
    }

    // Take field
    pub fn take_disk(&mut self) -> ::std::string::String {
        self.disk.take().unwrap_or_else(|| ::std::string::String::new())
    }

    pub fn get_disk(&self) -> &str {
        match self.disk.as_ref() {
            Some(v) => &v,
            None => "",
        }
    }

    fn get_disk_for_reflect(&self) -> &::protobuf::SingularField<::std::string::String> {
        &self.disk
    }

    fn mut_disk_for_reflect(&mut self) -> &mut ::protobuf::SingularField<::std::string::String> {
        &mut self.disk
    }
}

impl ::protobuf::Message for Operation {
    fn is_initialized(&self) -> bool {
        if self.Op_type.is_none() {
            return false;
        }
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream) -> ::protobuf::ProtobufResult<()> {
        while !is.eof()? {
            let (field_number, wire_type) = is.read_tag_unpack()?;
            match field_number {
                1 => {
                    if wire_type != ::protobuf::wire_format::WireTypeVarint {
                        return ::std::result::Result::Err(::protobuf::rt::unexpected_wire_type(wire_type));
                    }
                    let tmp = is.read_enum()?;
                    self.Op_type = ::std::option::Option::Some(tmp);
                },
                2 => {
                    ::protobuf::rt::read_singular_string_into(wire_type, is, &mut self.disk)?;
                },
                _ => {
                    ::protobuf::rt::read_unknown_or_skip_group(field_number, wire_type, is, self.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u32 {
        let mut my_size = 0;
        if let Some(v) = self.Op_type {
            my_size += ::protobuf::rt::enum_size(1, v);
        }
        if let Some(ref v) = self.disk.as_ref() {
            my_size += ::protobuf::rt::string_size(2, &v);
        }
        my_size += ::protobuf::rt::unknown_fields_size(self.get_unknown_fields());
        self.cached_size.set(my_size);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream) -> ::protobuf::ProtobufResult<()> {
        if let Some(v) = self.Op_type {
            os.write_enum(1, v.value())?;
        }
        if let Some(ref v) = self.disk.as_ref() {
            os.write_string(2, &v)?;
        }
        os.write_unknown_fields(self.get_unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn get_cached_size(&self) -> u32 {
        self.cached_size.get()
    }

    fn get_unknown_fields(&self) -> &::protobuf::UnknownFields {
        &self.unknown_fields
    }

    fn mut_unknown_fields(&mut self) -> &mut ::protobuf::UnknownFields {
        &mut self.unknown_fields
    }

    fn as_any(&self) -> &::std::any::Any {
        self as &::std::any::Any
    }
    fn as_any_mut(&mut self) -> &mut ::std::any::Any {
        self as &mut ::std::any::Any
    }
    fn into_any(self: Box<Self>) -> ::std::boxed::Box<::std::any::Any> {
        self
    }

    fn descriptor(&self) -> &'static ::protobuf::reflect::MessageDescriptor {
        ::protobuf::MessageStatic::descriptor_static(None::<Self>)
    }
}

impl ::protobuf::MessageStatic for Operation {
    fn new() -> Operation {
        Operation::new()
    }

    fn descriptor_static(_: ::std::option::Option<Operation>) -> &'static ::protobuf::reflect::MessageDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::MessageDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                let mut fields = ::std::vec::Vec::new();
                fields.push(::protobuf::reflect::accessor::make_option_accessor::<_, ::protobuf::types::ProtobufTypeEnum<Op>>(
                    "Op_type",
                    Operation::get_Op_type_for_reflect,
                    Operation::mut_Op_type_for_reflect,
                ));
                fields.push(::protobuf::reflect::accessor::make_singular_field_accessor::<_, ::protobuf::types::ProtobufTypeString>(
                    "disk",
                    Operation::get_disk_for_reflect,
                    Operation::mut_disk_for_reflect,
                ));
                ::protobuf::reflect::MessageDescriptor::new::<Operation>(
                    "Operation",
                    fields,
                    file_descriptor_proto()
                )
            })
        }
    }
}

impl ::protobuf::Clear for Operation {
    fn clear(&mut self) {
        self.clear_Op_type();
        self.clear_disk();
        self.unknown_fields.clear();
    }
}

impl ::std::fmt::Debug for Operation {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Operation {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Message(self)
    }
}

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
pub enum DiskType {
    SOLID_STATE = 0,
    ROTATIONAL = 1,
    LOOPBACK = 2,
    LVM = 3,
    RAM = 4,
    VIRTUAL = 5,
    UNKNOWN = 6,
}

impl ::protobuf::ProtobufEnum for DiskType {
    fn value(&self) -> i32 {
        *self as i32
    }

    fn from_i32(value: i32) -> ::std::option::Option<DiskType> {
        match value {
            0 => ::std::option::Option::Some(DiskType::SOLID_STATE),
            1 => ::std::option::Option::Some(DiskType::ROTATIONAL),
            2 => ::std::option::Option::Some(DiskType::LOOPBACK),
            3 => ::std::option::Option::Some(DiskType::LVM),
            4 => ::std::option::Option::Some(DiskType::RAM),
            5 => ::std::option::Option::Some(DiskType::VIRTUAL),
            6 => ::std::option::Option::Some(DiskType::UNKNOWN),
            _ => ::std::option::Option::None
        }
    }

    fn values() -> &'static [Self] {
        static values: &'static [DiskType] = &[
            DiskType::SOLID_STATE,
            DiskType::ROTATIONAL,
            DiskType::LOOPBACK,
            DiskType::LVM,
            DiskType::RAM,
            DiskType::VIRTUAL,
            DiskType::UNKNOWN,
        ];
        values
    }

    fn enum_descriptor_static(_: ::std::option::Option<DiskType>) -> &'static ::protobuf::reflect::EnumDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::EnumDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::EnumDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                ::protobuf::reflect::EnumDescriptor::new("DiskType", file_descriptor_proto())
            })
        }
    }
}

impl ::std::marker::Copy for DiskType {
}

impl ::protobuf::reflect::ProtobufValue for DiskType {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Enum(self.descriptor())
    }
}

#[derive(Clone,PartialEq,Eq,Debug,Hash)]
pub enum Op {
    Add = 1,
    List = 3,
    Remove = 4,
}

impl ::protobuf::ProtobufEnum for Op {
    fn value(&self) -> i32 {
        *self as i32
    }

    fn from_i32(value: i32) -> ::std::option::Option<Op> {
        match value {
            1 => ::std::option::Option::Some(Op::Add),
            3 => ::std::option::Option::Some(Op::List),
            4 => ::std::option::Option::Some(Op::Remove),
            _ => ::std::option::Option::None
        }
    }

    fn values() -> &'static [Self] {
        static values: &'static [Op] = &[
            Op::Add,
            Op::List,
            Op::Remove,
        ];
        values
    }

    fn enum_descriptor_static(_: ::std::option::Option<Op>) -> &'static ::protobuf::reflect::EnumDescriptor {
        static mut descriptor: ::protobuf::lazy::Lazy<::protobuf::reflect::EnumDescriptor> = ::protobuf::lazy::Lazy {
            lock: ::protobuf::lazy::ONCE_INIT,
            ptr: 0 as *const ::protobuf::reflect::EnumDescriptor,
        };
        unsafe {
            descriptor.get(|| {
                ::protobuf::reflect::EnumDescriptor::new("Op", file_descriptor_proto())
            })
        }
    }
}

impl ::std::marker::Copy for Op {
}

impl ::protobuf::reflect::ProtobufValue for Op {
    fn as_ref(&self) -> ::protobuf::reflect::ProtobufValueRef {
        ::protobuf::reflect::ProtobufValueRef::Enum(self.descriptor())
    }
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\x14protos/service.proto\x12\tceph_disk\"n\n\x03Osd\x12\x0c\n\x04fsid\
    \x18\x01\x20\x01(\t\x12\n\n\x02id\x18\x02\x20\x02(\x04\x12\x14\n\x0cbloc\
    k_device\x18\x03\x20\x01(\t\x12\x0e\n\x06active\x18\x04\x20\x02(\x08\x12\
    \x12\n\nused_space\x18\x05\x20\x02(\x04\x12\x13\n\x0btotal_space\x18\x06\
    \x20\x02(\x04\"R\n\x04Disk\x12!\n\x04type\x18\x01\x20\x02(\x0e2\x13.ceph\
    _disk.DiskType\x12\x10\n\x08dev_path\x18\x02\x20\x02(\t\x12\x15\n\rseria\
    l_number\x18\x03\x20\x01(\t\"&\n\x05Disks\x12\x1d\n\x04disk\x18\x01\x20\
    \x03(\x0b2\x0f.ceph_disk.Disk\"l\n\x08OpResult\x12.\n\x06result\x18\x01\
    \x20\x02(\x0e2\x1e.ceph_disk.OpResult.ResultType\x12\x11\n\terror_msg\
    \x18\x02\x20\x01(\t\"\x1d\n\nResultType\x12\x06\n\x02OK\x10\0\x12\x07\n\
    \x03ERR\x10\x01\"9\n\tOperation\x12\x1e\n\x07Op_type\x18\x01\x20\x02(\
    \x0e2\r.ceph_disk.Op\x12\x0c\n\x04disk\x18\x02\x20\x01(\t*e\n\x08DiskTyp\
    e\x12\x0f\n\x0bSOLID_STATE\x10\0\x12\x0e\n\nROTATIONAL\x10\x01\x12\x0c\n\
    \x08LOOPBACK\x10\x02\x12\x07\n\x03LVM\x10\x03\x12\x07\n\x03RAM\x10\x04\
    \x12\x0b\n\x07VIRTUAL\x10\x05\x12\x0b\n\x07UNKNOWN\x10\x06*#\n\x02Op\x12\
    \x07\n\x03Add\x10\x01\x12\x08\n\x04List\x10\x03\x12\n\n\x06Remove\x10\
    \x04B\x02H\x01\
";

static mut file_descriptor_proto_lazy: ::protobuf::lazy::Lazy<::protobuf::descriptor::FileDescriptorProto> = ::protobuf::lazy::Lazy {
    lock: ::protobuf::lazy::ONCE_INIT,
    ptr: 0 as *const ::protobuf::descriptor::FileDescriptorProto,
};

fn parse_descriptor_proto() -> ::protobuf::descriptor::FileDescriptorProto {
    ::protobuf::parse_from_bytes(file_descriptor_proto_data).unwrap()
}

pub fn file_descriptor_proto() -> &'static ::protobuf::descriptor::FileDescriptorProto {
    unsafe {
        file_descriptor_proto_lazy.get(|| {
            parse_descriptor_proto()
        })
    }
}
