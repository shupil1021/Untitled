//! 와이파이 상태 — 연결 여부, SSID, 로컬 IP 를 Windows 에서 조회한다(작업표시줄의 와이파이 아이콘과 클릭 팝업용).

// 작업표시줄 와이파이 아이콘용 — 실제 이 PC 의 인터넷 연결 상태를 물어본다.
// (와이파이인지 유선인지까지는 구분 안 하고, 그냥 "연결돼 있는지"만 확인)
pub(super) fn network_connected() -> bool {
    use windows::Win32::Networking::WinInet::{InternetGetConnectedState, INTERNET_CONNECTION};
    let mut flags = INTERNET_CONNECTION(0);
    unsafe { InternetGetConnectedState(&mut flags, 0) }.is_ok()
}

// 와이파이 아이콘을 눌렀을 때 보여줄 상세 정보. 아이콘을 열 때 한 번만 조회한다
// (WLAN 조회는 매 프레임 부를 만큼 가볍지 않다).
pub(super) struct WifiInfo {
    pub(super) ssid: Option<String>,
    pub(super) ip: Option<String>,
}

pub(super) fn query_wifi_info(connected: bool) -> WifiInfo {
    WifiInfo { ssid: if connected { current_ssid() } else { None }, ip: local_ip() }
}

// 소켓을 직접 열어서 IP 를 알아내면(UDP connect 트릭 등) 처음 실행할 때 Windows
// 방화벽이 "이 앱의 네트워크 접근을 허용할까요?" 팝업을 띄운다 — 소켓/실제 통신을
// 아예 안 하고 OS 가 이미 들고 있는 어댑터 설정을 순수 조회만 하는 IP Helper API
// (GetAdaptersAddresses) 를 쓰면 그 팝업 자체가 안 뜬다.
pub(super) fn local_ip() -> Option<String> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
        IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};
    unsafe {
        let family = AF_INET.0 as u32;
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
        let mut size = 0u32;
        // 첫 호출은 필요한 버퍼 크기만 알아내려는 것 — 항상 실패(버퍼 부족)한다.
        let _ = GetAdaptersAddresses(family, flags, None, None, &mut size);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let list = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        if GetAdaptersAddresses(family, flags, None, Some(list), &mut size) != 0 {
            return None;
        }
        let mut cur = list;
        while !cur.is_null() {
            let adapter = &*cur;
            if adapter.OperStatus == IfOperStatusUp {
                let mut ua = adapter.FirstUnicastAddress;
                while !ua.is_null() {
                    let sa = (*ua).Address.lpSockaddr;
                    if !sa.is_null() && (*sa).sa_family == AF_INET {
                        let sin = sa as *const SOCKADDR_IN;
                        let b = (*sin).sin_addr.S_un.S_un_b;
                        return Some(format!("{}.{}.{}.{}", b.s_b1, b.s_b2, b.s_b3, b.s_b4));
                    }
                    ua = (*ua).Next;
                }
            }
            cur = adapter.Next;
        }
        None
    }
}

// 현재 연결된 와이파이 인터페이스의 SSID 를 WLAN API 로 조회. 실패하거나 와이파이가
// 아니면(유선 등) None.
pub(super) fn current_ssid() -> Option<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::NetworkManagement::WiFi::*;
    unsafe {
        let mut handle = HANDLE::default();
        let mut negotiated = 0u32;
        if WlanOpenHandle(2, None, &mut negotiated, &mut handle) != 0 {
            return None;
        }
        let mut list_ptr: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        if WlanEnumInterfaces(handle, None, &mut list_ptr) != 0 || list_ptr.is_null() {
            let _ = WlanCloseHandle(handle, None);
            return None;
        }
        let n = (*list_ptr).dwNumberOfItems as usize;
        let infos = std::slice::from_raw_parts((*list_ptr).InterfaceInfo.as_ptr(), n);
        let mut ssid = None;
        for info in infos {
            let mut data_size = 0u32;
            let mut data_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
            let ok = WlanQueryInterface(
                handle,
                &info.InterfaceGuid,
                wlan_intf_opcode_current_connection,
                None,
                &mut data_size,
                &mut data_ptr,
                None,
            );
            if ok == 0 && !data_ptr.is_null() {
                let attrs = &*(data_ptr as *const WLAN_CONNECTION_ATTRIBUTES);
                let raw = attrs.wlanAssociationAttributes.dot11Ssid;
                let len = (raw.uSSIDLength as usize).min(32);
                if len > 0 {
                    ssid = String::from_utf8(raw.ucSSID[..len].to_vec()).ok();
                }
                WlanFreeMemory(data_ptr);
                if ssid.is_some() {
                    break;
                }
            }
        }
        WlanFreeMemory(list_ptr as *const _);
        let _ = WlanCloseHandle(handle, None);
        ssid
    }
}
