use bitflags::bitflags;
use crate::enums::constants::*;
use nom7::number::complete::{*,{le_u16, le_u32, le_u8}};
use nom7::bytes::streaming::take;
use nom7::combinator::cond;
use nom7::multi::count;
use nom7::IResult;

// https://github.com/fox-it/dissect.cstruct/blob/master/examples/secdesc.py
// http://www.selfadsi.org/deep-inside/ad-security-descriptors.htm#SecurityDescriptorStructure
// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/20233ed8-a6c6-4097-aafa-dd545ed24428?redirectedfrom=MSDN
// https://github.com/OISF/suricata/blob/master/rust/src/dcerpc/parser.rs

/// Structure for Security Descriptor network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/7d4dac05-9cef-4563-a058-f108abecce1d>
#[derive(Debug)]
pub struct SecurityDescriptor {
    pub revision: u8,
    pub sbz1: u8,
    pub control: u16,
    pub offset_owner: u32,
    pub offset_group: u32,
    pub offset_sacl: u32,
    pub offset_dacl: u32,
}

impl SecurityDescriptor {
    pub fn parse(i: &[u8]) -> IResult<&[u8], SecurityDescriptor>
    {
        let (i, revision) = le_u8(i)?;
        let (i, sbz1) = le_u8(i)?;
        let (i, control) = le_u16(i)?;
        let (i, offset_owner) = le_u32(i)?;
        let (i, offset_group) = le_u32(i)?;
        let (i, offset_sacl) = le_u32(i)?;
        let (i, offset_dacl) = le_u32(i)?;

        let nt = SecurityDescriptor {
            revision: revision,
            sbz1: sbz1,
            control: control,
            offset_owner: offset_owner,
            offset_group: offset_group,
            offset_sacl: offset_sacl,
            offset_dacl: offset_dacl,
        };
        Ok((i, nt))
    }
}

/// Strcuture for Sid Identified Authority network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/c6ce4275-3d90-4890-ab3a-514745e4637e>
#[derive(Debug, Clone)]
pub struct LdapSidIdentifiedAuthority {
    pub value: Vec<u8>,
}

impl LdapSidIdentifiedAuthority {
    pub fn parse(i: &[u8]) -> IResult<&[u8], LdapSidIdentifiedAuthority>
    {
        let (i, value) = take(6_usize)(i)?;

        let sid_authority = LdapSidIdentifiedAuthority {
            value: value.to_vec(),
        };
        Ok((i, sid_authority))
    }
}

/// Structure for LDAPSID network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/f992ad60-0fe4-4b87-9fed-beb478836861>
#[derive(Clone, Debug)]
pub struct LdapSid {
    pub revision: u8,
    pub sub_authority_count: u8,
    pub identifier_authority: LdapSidIdentifiedAuthority,
    pub sub_authority: Vec<u32>,
}

impl LdapSid {
    pub fn parse(i: &[u8]) -> IResult<&[u8], LdapSid>
    {
        let (i, revision) = le_u8(i)?;
        let (i, sub_authority_count) = le_u8(i)?;
        let (i, identifier_authority) = LdapSidIdentifiedAuthority::parse(i)?;
        let (i, sub_authority) = count(le_u32, sub_authority_count as usize)(i)?;

        let ldap_sid = LdapSid {
            revision: revision,
            sub_authority_count: sub_authority_count,
            identifier_authority: identifier_authority,
            sub_authority: sub_authority.to_vec(),
        };
        Ok((i, ldap_sid))
    }
}

/// Structure for Acl network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/20233ed8-a6c6-4097-aafa-dd545ed24428>
#[derive(Debug)]
pub struct Acl {
    pub acl_revision: u8,
    pub sbz1: u8,
    pub acl_size: u16,
    pub ace_count: u16,
    pub sbz2: u16,
    // Length = acl_size
    pub data: Vec<Ace>,
}

impl Acl {
    pub fn parse(i: &[u8]) -> IResult<&[u8], Acl>
    {
        let (i, acl_revision) = le_u8(i)?;
        let (i, sbz1) = le_u8(i)?;
        let (i, acl_size) = le_u16(i)?;
        let (i, ace_count) = le_u16(i)?;
        let (i, sbz2) = le_u16(i)?;
        let (i, data) = count(Ace::parse, ace_count as usize)(i)?;

        let acl = Acl {
            acl_revision: acl_revision,
            sbz1: sbz1,
            acl_size: acl_size,
            ace_count: ace_count,
            sbz2: sbz2,
            data: data,
        };
        Ok((i, acl))
    }
}

/// Structure for Ace network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/628ebb1d-c509-4ea0-a10f-77ef97ca4586>
#[derive(Debug)]
pub struct Ace {
    pub ace_type: u8,
    pub ace_flags: u8,
    pub ace_size: u16,
    pub data: AceFormat,
}

impl Ace {
    pub fn parse(i: &[u8]) -> IResult<&[u8], Ace>
    {
        let (i, ace_type) = le_u8(i)?;
        let (i, ace_flags) = le_u8(i)?;
        let (i, ace_size) = le_u16(i)?;
        let (i, data) = take(ace_size as usize - 4)(i)?;
        let (_j,ace_data_formated) = AceFormat::parse(data, ace_type)?;

        let ace = Ace {
            ace_type: ace_type,
            ace_flags: ace_flags,
            ace_size: ace_size,
            data: ace_data_formated,
        };
        Ok((i, ace))
    }
}

/// Enum to get the same ouput for data switch in Ace structure.
#[derive(Clone, Debug)]
pub enum AceFormat {
    AceAllowed(AccessAllowedAce),
    AceObjectAllowed(AccessAllowedObjectAce),
    Empty,
}

impl AceFormat {
    pub fn parse(i: &[u8], ace_type: u8) -> IResult<&[u8], AceFormat>
    {
        if &ace_type == &ACCESS_ALLOWED_ACE_TYPE {
            let data = AceFormat::AceAllowed(AccessAllowedAce::parse(i)?.1);
            Ok((i, data))
        }
        else if &ace_type == &ACCESS_DENIED_ACE_TYPE { 
            let data = AceFormat::AceAllowed(AccessAllowedAce::parse(i)?.1);
            Ok((i, data))
        }
        else if &ace_type == &ACCESS_ALLOWED_OBJECT_ACE_TYPE {
            let data = AceFormat::AceObjectAllowed(AccessAllowedObjectAce::parse(i)?.1);
            Ok((i, data))
        }
        else if &ace_type == &ACCESS_DENIED_OBJECT_ACE_TYPE { 
            let data = AceFormat::AceObjectAllowed(AccessAllowedObjectAce::parse(i)?.1);
            Ok((i, data))
        }
        else {
            panic!("Error during ACE data parsing to AceFormat!")
        }
    }
    
    pub fn get_mask(value: AceFormat) -> Option<u32>
    {
        match value {
            AceFormat::AceAllowed(ace) => Some(ace.mask),
            AceFormat::AceObjectAllowed(ace) => Some(ace.mask),
            AceFormat::Empty => None,
        }
    }

    pub fn get_sid(value: AceFormat) -> Option<LdapSid>
    {
        match value {
            AceFormat::AceAllowed(ace) => Some(ace.sid),
            AceFormat::AceObjectAllowed(ace) => Some(ace.sid),
            AceFormat::Empty => None,
        }
    }

    pub fn get_flags(value: AceFormat) -> Option<ObjectAceFlags>
    {
        match value {
            AceFormat::AceAllowed(_) => None,
            AceFormat::AceObjectAllowed(ace) => Some(ace.flags),
            AceFormat::Empty => None,
        }
    }

    pub fn get_object_type(value: AceFormat) -> Option<u128>
    {
        match value {
            AceFormat::AceAllowed(_) => None,
            AceFormat::AceObjectAllowed(ace) => ace.object_type,
            AceFormat::Empty => None,
        }
    }

    pub fn get_inherited_object_type(value: AceFormat) -> Option<u128>
    {
        match value {
            AceFormat::AceAllowed(_) => None,
            AceFormat::AceObjectAllowed(ace) => ace.inherited_object_type,
            AceFormat::Empty => None,
        }
    }
}

/// Structure for Access Allowed Ace network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/72e7c7ea-bc02-4c74-a619-818a16bf6adb>
#[derive(Clone, Debug)]
pub struct AccessAllowedAce {
    pub mask: u32,
    pub sid: LdapSid,
}

impl AccessAllowedAce {
    pub fn parse(i: &[u8]) -> IResult<&[u8], AccessAllowedAce>
    {
        let (i, mask) = le_u32(i)?;
        let (i, sid) = LdapSid::parse(i)?;

        let access_allowed_ace = AccessAllowedAce {
            mask: mask,
            sid: sid,
        };
        Ok((i, access_allowed_ace))
    }
}

/// Structure for Access Allowed Object Ace network packet.
/// <https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/c79a383c-2b3f-4655-abe7-dcbb7ce0cfbe>
#[derive(Clone, Debug)]
pub struct AccessAllowedObjectAce {
    pub mask: u32,
    pub flags: ObjectAceFlags,
    pub object_type: Option<u128>,
    pub inherited_object_type: Option<u128>,
    pub sid: LdapSid,
}

impl AccessAllowedObjectAce {
    pub fn parse(i: &[u8]) -> IResult<&[u8], AccessAllowedObjectAce>
    {
        let (i, mask) = le_u32(i)?;
        let (i, flags) = ObjectAceFlags::parse(i)?;
        let (i, object_type) = cond(flags.contains(ObjectAceFlags::ACE_OBJECT_PRESENT),le_u128)(i)?;
        let (i, inherited_object_type) = cond(flags.contains(ObjectAceFlags::ACE_INHERITED_OBJECT_PRESENT),le_u128)(i)?;
        let (i, sid) = LdapSid::parse(i)?;

        let access_allowed_object_ace = AccessAllowedObjectAce {
            mask: mask,
            flags: flags,
            object_type: object_type,
            inherited_object_type: inherited_object_type,
            sid: sid,
        };
        Ok((i, access_allowed_object_ace))
    }
}

bitflags! {
    /// AceFlags
    pub struct ObjectAceFlags : u32 {
        const ACE_OBJECT_PRESENT = 0x0001;
        const ACE_INHERITED_OBJECT_PRESENT = 0x0002;
    }
}

impl ObjectAceFlags {
    pub fn parse(i: &[u8]) -> IResult<&[u8], ObjectAceFlags>
    {
        let (i, flags) = le_u32(i)?;
        let object_ace_flags = ObjectAceFlags::from_bits(flags).unwrap();
        Ok((i, object_ace_flags))
    }
}

/// Test functions
#[test]
#[rustfmt::skip]
pub fn test_security_descriptor() {

    let original = vec![
        // SECURITY_DECRIPTOR [0..15]
            // revision
            1,
            // Internal
            0,
            // control flags
            4, 140,
            // offset_owner
            120, 9, 0, 0,
            // offset_group
            0, 0, 0, 0,
            // offset_sacl
            0, 0, 0, 0,
            // offset_dacl
            20, 0, 0, 0
    ];

    let nt = SecurityDescriptor::parse(&original).unwrap().1;
    assert_eq!(nt.revision, 1);
    assert_eq!(nt.sbz1, 0);
    assert_eq!(nt.control, 35844);
    assert_eq!(nt.offset_owner, 2424);
    assert_eq!(nt.offset_group, 0);
    assert_eq!(nt.offset_sacl, 0);
    assert_eq!(nt.offset_dacl, 20);
    
    println!("[NT SecurityDescriptor]: {:?}",&nt)
}

#[test]
#[rustfmt::skip]
pub fn test_ace() {

    let original_ace = vec![
        // Type
        0x00,
        // Flag
        0x12,
        // Size
        0x18, 0x00,
        // Data
            // Mask
            0xbd, 0x01, 0x0f, 0x00,
            // Sid
            0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x20, 0x02, 0x00, 0x00
    ];

    let result          = Ace::parse(&original_ace).unwrap().1;
    assert_eq!(result.ace_type, 0);
    println!("ACE_ALLOWED: {:?}",result);


    let original_ace_object = vec![
        // Type
        0x05,
        // Flag
        0x12,
        // Size
        0x2c, 0x00,
        // Data
            // Mask
            0x94, 0x00, 0x02, 0x00,
            // Ace Object
                // Flags
                0x02, 0x00, 0x00, 0x00,
                // Inherited GUID
                0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2,
            // Sid
            0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00
    ];

    let result          = Ace::parse(&original_ace_object).unwrap().1;
    assert_eq!(result.ace_type, 5);
    println!("ACE_ALLOWED_OBJECT: {:?}",result);
}

#[test]
#[rustfmt::skip]
pub fn test_acl_admin() {

    //Adminstrateur test Acl
    //let original_acl = vec![ 0x04, 0x00, 0x74, 0x04, 0x18, 0x00, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x42, 0x16, 0x4c, 0xc0, 0x20, 0xd0, 0x11, 0xa7, 0x68, 0x00, 0xaa, 0x00, 0x6e, 0x05, 0x29, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x42, 0x16, 0x4c, 0xc0, 0x20, 0xd0, 0x11, 0xa7, 0x68, 0x00, 0xaa, 0x00, 0x6e, 0x05, 0x29, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20, 0x20, 0x5f, 0xa5, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20, 0x20, 0x5f, 0xa5, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x40, 0xc2, 0x0a, 0xbc, 0xa9, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x40, 0xc2, 0x0a, 0xbc, 0xa9, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x42, 0x2f, 0xba, 0x59, 0xa2, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd3, 0xcf, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x42, 0x2f, 0xba, 0x59, 0xa2, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd3, 0xcf, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xf8, 0x88, 0x70, 0x03, 0xe1, 0x0a, 0xd2, 0x11, 0xb4, 0x22, 0x00, 0xa0, 0xc9, 0x68, 0xf9, 0x39, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xf8, 0x88, 0x70, 0x03, 0xe1, 0x0a, 0xd2, 0x11, 0xb4, 0x22, 0x00, 0xa0, 0xc9, 0x68, 0xf9, 0x39, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x38, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x7f, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xe8, 0xc0, 0xf8, 0x7a, 0xfa, 0x0f, 0x9e, 0xac, 0x5c, 0xef, 0xbe, 0x73, 0x05, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1d, 0xb1, 0xa9, 0x46, 0xae, 0x60, 0x5a, 0x40, 0xb7, 0xe8, 0xff, 0x8a, 0x58, 0xd4, 0x56, 0xd2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x30, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1c, 0x9a, 0xb6, 0x6d, 0x22, 0x94, 0xd1, 0x11, 0xae, 0xbd, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x31, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x62, 0xbc, 0x05, 0x58, 0xc9, 0xbd, 0x28, 0x44, 0xa5, 0xe2, 0x85, 0x6a, 0x0f, 0x4c, 0x18, 0x5e, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x31, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x94, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x94, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x53, 0x1a, 0x72, 0xab, 0x2f, 0x1e, 0xd0, 0x11, 0x98, 0x19, 0x00, 0xaa, 0x00, 0x40, 0x52, 0x9b, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x53, 0x1a, 0x72, 0xab, 0x2f, 0x1e, 0xd0, 0x11, 0x98, 0x19, 0x00, 0xaa, 0x00, 0x40, 0x52, 0x9b, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x05, 0x02, 0x28, 0x00, 0x30, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xde, 0x47, 0xe6, 0x91, 0x6f, 0xd9, 0x70, 0x4b, 0x95, 0x57, 0xd6, 0x3f, 0xf4, 0xf3, 0xcc, 0xd8, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00, 0xbf, 0x01, 0x0e, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xe8, 0xc0, 0xf8, 0x7a, 0xfa, 0x0f, 0x9e, 0xac, 0x5c, 0xef, 0xbe, 0x73, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00, 0xbf, 0x01, 0x0e, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xe8, 0xc0, 0xf8, 0x7a, 0xfa, 0x0f, 0x9e, 0xac, 0x5c, 0xef, 0xbe, 0x73, 0x07, 0x02, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0xbf, 0x01, 0x0f, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x20, 0x02, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x94, 0x00, 0x02, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0xff, 0x01, 0x0f, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x00, 0x00, 0x00 ];
    //let result = Acl::parse(&original_acl).unwrap().1;
    //assert_eq!(result.acl_size, 1140);

    //Guest (with null bytes)
    let original_acl = vec![0x04, 0x00, 0x04, 0x0b, 0x34, 0x00, 0x00, 0x00, 0x05, 0x00, 0x38, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x42, 0x16, 0x4c, 0xc0, 0x20, 0xd0, 0x11, 0xa7, 0x68, 0x00, 0xaa, 0x00, 0x6e, 0x05, 0x29, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x29, 0x02, 0x00, 0x00, 0x05, 0x00, 0x38, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x20, 0x20, 0x5f, 0xa5, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x29, 0x02, 0x00, 0x00, 0x05, 0x00, 0x38, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x40, 0xc2, 0x0a, 0xbc, 0xa9, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x29, 0x02, 0x00, 0x00, 0x05, 0x00, 0x38, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xf8, 0x88, 0x70, 0x03, 0xe1, 0x0a, 0xd2, 0x11, 0xb4, 0x22, 0x00, 0xa0, 0xc9, 0x68, 0xf9, 0x39, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x29, 0x02, 0x00, 0x00, 0x05, 0x00, 0x38, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x7f, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x05, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1d, 0xb1, 0xa9, 0x46, 0xae, 0x60, 0x5a, 0x40, 0xb7, 0xe8, 0xff, 0x8a, 0x58, 0xd4, 0x56, 0xd2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x30, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1c, 0x9a, 0xb6, 0x6d, 0x22, 0x94, 0xd1, 0x11, 0xae, 0xbd, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x31, 0x02, 0x00, 0x00, 0x05, 0x00, 0x2c, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x62, 0xbc, 0x05, 0x58, 0xc9, 0xbd, 0x28, 0x44, 0xa5, 0xe2, 0x85, 0x6a, 0x0f, 0x4c, 0x18, 0x5e, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x31, 0x02, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x53, 0x1a, 0x72, 0xab, 0x2f, 0x1e, 0xd0, 0x11, 0x98, 0x19, 0x00, 0xaa, 0x00, 0x40, 0x52, 0x9b, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x53, 0x1a, 0x72, 0xab, 0x2f, 0x1e, 0xd0, 0x11, 0x98, 0x19, 0x00, 0xaa, 0x00, 0x40, 0x52, 0x9b, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x54, 0x1a, 0x72, 0xab, 0x2f, 0x1e, 0xd0, 0x11, 0x98, 0x19, 0x00, 0xaa, 0x00, 0x40, 0x52, 0x9b, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x56, 0x1a, 0x72, 0xab, 0x2f, 0x1e, 0xd0, 0x11, 0x98, 0x19, 0x00, 0xaa, 0x00, 0x40, 0x52, 0x9b, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x42, 0x2f, 0xba, 0x59, 0xa2, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd3, 0xcf, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0b, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x54, 0x01, 0x8d, 0xe4, 0xf8, 0xbc, 0xd1, 0x11, 0x87, 0x02, 0x00, 0xc0, 0x4f, 0xb9, 0x60, 0x50, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0b, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x86, 0xb8, 0xb5, 0x77, 0x4a, 0x94, 0xd1, 0x11, 0xae, 0xbd, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0b, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb3, 0x95, 0x57, 0xe4, 0x55, 0x94, 0xd1, 0x11, 0xae, 0xbd, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0b, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x86, 0xb8, 0xb5, 0x77, 0x4a, 0x94, 0xd1, 0x11, 0xae, 0xbd, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb2, 0x95, 0x57, 0xe4, 0x55, 0x94, 0xd1, 0x11, 0xae, 0xbd, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x05, 0x00, 0x28, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb3, 0x95, 0x57, 0xe4, 0x55, 0x94, 0xd1, 0x11, 0xae, 0xbd, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00, 0xff, 0x01, 0x0f, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0xff, 0x01, 0x0f, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x24, 0x02, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x94, 0x00, 0x02, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0xff, 0x01, 0x0f, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x00, 0x00, 0x00, 0x05, 0x1a, 0x5c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x42, 0x16, 0x4c, 0xc0, 0x20, 0xd0, 0x11, 0xa7, 0x68, 0x00, 0xaa, 0x00, 0x6e, 0x05, 0x29, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x42, 0x16, 0x4c, 0xc0, 0x20, 0xd0, 0x11, 0xa7, 0x68, 0x00, 0xaa, 0x00, 0x6e, 0x05, 0x29, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x1a, 0x5c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20, 0x20, 0x5f, 0xa5, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x10, 0x20, 0x20, 0x5f, 0xa5, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x1a, 0x5c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x40, 0xc2, 0x0a, 0xbc, 0xa9, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x40, 0xc2, 0x0a, 0xbc, 0xa9, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd4, 0xcf, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x1a, 0x5c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x42, 0x2f, 0xba, 0x59, 0xa2, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd3, 0xcf, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x42, 0x2f, 0xba, 0x59, 0xa2, 0x79, 0xd0, 0x11, 0x90, 0x20, 0x00, 0xc0, 0x4f, 0xc2, 0xd3, 0xcf, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x1a, 0x5c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xf8, 0x88, 0x70, 0x03, 0xe1, 0x0a, 0xd2, 0x11, 0xb4, 0x22, 0x00, 0xa0, 0xc9, 0x68, 0xf9, 0x39, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x3c, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xf8, 0x88, 0x70, 0x03, 0xe1, 0x0a, 0xd2, 0x11, 0xb4, 0x22, 0x00, 0xa0, 0xc9, 0x68, 0xf9, 0x39, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x1a, 0x58, 0x00, 0x08, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xa6, 0x6d, 0x02, 0x9b, 0x3c, 0x0d, 0x5c, 0x46, 0x8b, 0xee, 0x51, 0x99, 0xd7, 0x16, 0x5c, 0xba, 0x86, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x1a, 0x58, 0x00, 0x08, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xa6, 0x6d, 0x02, 0x9b, 0x3c, 0x0d, 0x5c, 0x46, 0x8b, 0xee, 0x51, 0x99, 0xd7, 0x16, 0x5c, 0xba, 0x86, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x1a, 0x58, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x6d, 0x9e, 0xc6, 0xb7, 0xc7, 0x2c, 0xd2, 0x11, 0x85, 0x4e, 0x00, 0xa0, 0xc9, 0x83, 0xf6, 0x08, 0x86, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x1a, 0x58, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x6d, 0x9e, 0xc6, 0xb7, 0xc7, 0x2c, 0xd2, 0x11, 0x85, 0x4e, 0x00, 0xa0, 0xc9, 0x83, 0xf6, 0x08, 0x9c, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x38, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x6d, 0x9e, 0xc6, 0xb7, 0xc7, 0x2c, 0xd2, 0x11, 0x85, 0x4e, 0x00, 0xa0, 0xc9, 0x83, 0xf6, 0x08, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x09, 0x00, 0x00, 0x00, 0x05, 0x1a, 0x58, 0x00, 0x20, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x93, 0x7b, 0x1b, 0xea, 0x48, 0x5e, 0xd5, 0x46, 0xbc, 0x6c, 0x4d, 0xf4, 0xfd, 0xa7, 0x8a, 0x35, 0x86, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x1a, 0x4c, 0x00, 0x94, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x14, 0xcc, 0x28, 0x48, 0x37, 0x14, 0xbc, 0x45, 0x9b, 0x07, 0xad, 0x6f, 0x01, 0x5e, 0x5f, 0x28, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x1a, 0x4c, 0x00, 0x94, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x9c, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x12, 0x2c, 0x00, 0x94, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0xba, 0x7a, 0x96, 0xbf, 0xe6, 0x0d, 0xd0, 0x11, 0xa2, 0x85, 0x00, 0xaa, 0x00, 0x30, 0x49, 0xe2, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x05, 0x12, 0x28, 0x00, 0x30, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xde, 0x47, 0xe6, 0x91, 0x6f, 0xd9, 0x70, 0x4b, 0x95, 0x57, 0xd6, 0x3f, 0xf4, 0xf3, 0xcc, 0xd8, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x05, 0x12, 0x38, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0f, 0xd6, 0x47, 0x5b, 0x90, 0x60, 0xb2, 0x40, 0x9f, 0x37, 0x2a, 0x4d, 0xe8, 0x8f, 0x30, 0x63, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x0e, 0x02, 0x00, 0x00, 0x05, 0x12, 0x38, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0f, 0xd6, 0x47, 0x5b, 0x90, 0x60, 0xb2, 0x40, 0x9f, 0x37, 0x2a, 0x4d, 0xe8, 0x8f, 0x30, 0x63, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x0f, 0x02, 0x00, 0x00, 0x05, 0x12, 0x38, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xbe, 0x3b, 0x0e, 0xf3, 0xf0, 0x9f, 0xd1, 0x11, 0xb6, 0x03, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x52, 0x04, 0x00, 0x00, 0x05, 0x12, 0x38, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xbf, 0x3b, 0x0e, 0xf3, 0xf0, 0x9f, 0xd1, 0x11, 0xb6, 0x03, 0x00, 0x00, 0xf8, 0x03, 0x67, 0xc1, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x52, 0x04, 0x00, 0x00, 0x05, 0x12, 0x28, 0x00, 0x30, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xe5, 0xc3, 0x78, 0x3f, 0x9a, 0xf7, 0xbd, 0x46, 0xa0, 0xb8, 0x9d, 0x18, 0x11, 0x6d, 0xdc, 0x79, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x12, 0x24, 0x00, 0xff, 0x01, 0x0f, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x15, 0x00, 0x00, 0x00, 0xc1, 0x20, 0x09, 0x42, 0x30, 0x14, 0x92, 0xf5, 0x91, 0xfe, 0x95, 0x5f, 0x07, 0x02, 0x00, 0x00, 0x00, 0x12, 0x18, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x2a, 0x02, 0x00, 0x00, 0x00, 0x12, 0x18, 0x00, 0xbd, 0x01, 0x0f, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x20, 0x00, 0x00, 0x00, 0x20, 0x02, 0x00, 0x00];
    let acl =  Acl::parse(&original_acl).unwrap().1;

    assert_eq!(acl.acl_revision, 4);
    assert_eq!(acl.acl_size, 2820);
    assert_eq!(acl.ace_count, 52);

    //print result
    println!("ACL: {:?}",&acl.data);
    let mut count = 1;
    for ace in &acl.data {
        println!("[{}: ACE] {:?}\n",count,ace);
        println!("[{} ACE.DATA] {:?}\n", count, &ace.data);
        count +=1;
    }
}