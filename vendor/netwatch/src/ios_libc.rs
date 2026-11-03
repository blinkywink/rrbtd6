//! RustTD6: `libc` leaves the routing constants out for iOS, though iOS has the same Darwin kernel values as macOS.
//! The modules using them import this as `libc` on iOS.

pub(crate) use ::libc::*;

pub(crate) const NET_RT_DUMP: ::libc::c_int = 1;
pub(crate) const RTF_IFSCOPE: ::libc::c_int = 0x1000000;
pub(crate) const RTM_VERSION: ::libc::c_int = 5;
pub(crate) const RTM_LOCK: ::libc::c_int = 0x8;
pub(crate) const RTM_RESOLVE: ::libc::c_int = 0xb;
pub(crate) const RTM_NEWADDR: ::libc::c_int = 0xc;
pub(crate) const RTM_DELADDR: ::libc::c_int = 0xd;
pub(crate) const RTM_IFINFO: ::libc::c_int = 0xe;
pub(crate) const RTM_NEWMADDR: ::libc::c_int = 0xf;
pub(crate) const RTM_DELMADDR: ::libc::c_int = 0x10;
pub(crate) const RTM_IFINFO2: ::libc::c_int = 0x12;
pub(crate) const RTM_NEWMADDR2: ::libc::c_int = 0x13;
pub(crate) const RTM_GET2: ::libc::c_int = 0x14;
pub(crate) const RTAX_MAX: ::libc::c_int = 8;
pub(crate) const RTF_GATEWAY: ::libc::c_int = 0x2;
pub(crate) const RTM_ADD: ::libc::c_int = 0x1;
pub(crate) const RTM_DELETE: ::libc::c_int = 0x2;
pub(crate) const RTM_CHANGE: ::libc::c_int = 0x3;
pub(crate) const RTM_GET: ::libc::c_int = 0x4;
pub(crate) const RTM_LOSING: ::libc::c_int = 0x5;
pub(crate) const RTM_REDIRECT: ::libc::c_int = 0x6;
pub(crate) const RTM_MISS: ::libc::c_int = 0x7;
pub(crate) const RTA_IFP: ::libc::c_int = 0x10;
pub(crate) const RTAX_DST: ::libc::c_int = 0;
pub(crate) const RTAX_GATEWAY: ::libc::c_int = 1;
pub(crate) const RTAX_NETMASK: ::libc::c_int = 2;
pub(crate) const RTAX_IFP: ::libc::c_int = 4;
pub(crate) const RTAX_BRD: ::libc::c_int = 7;
