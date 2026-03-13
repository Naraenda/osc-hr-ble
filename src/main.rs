use std::error::Error;
use futures_lite::StreamExt;

#[derive(Debug, PartialEq)]
pub struct HeartRateMeasurement {
    pub heart_rate: u32,
    pub energy_expended: Option<u16>,
    pub rr_intervals: Option<Vec<u16>>,
}

impl HeartRateMeasurement {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        fn consume<const N: usize>(cursor: &mut &[u8]) -> Result<[u8; N], ()> {
            let (head, tail) = cursor.split_at_checked(N).ok_or(())?;
            *cursor = tail;
            head.try_into().map_err(|_| ())
        }

        let (&flags, mut cursor) = bytes.split_first()?;
        
        let is_u16 = (flags & 0x01) != 0;
        let has_ee = (flags & 0x08) != 0;
        let has_rr = (flags & 0x10) != 0;

        let heart_rate = if is_u16 {
            u16::from_le_bytes(consume(&mut cursor).ok()?) as u32
        } else {
            u8::from_le_bytes(consume(&mut cursor).ok()?) as u32
        };

        let energy_expended = has_ee
            .then(|| consume(&mut cursor).map(u16::from_le_bytes))
            .transpose()
            .ok()?;

        let rr_intervals = has_rr.then(||
            cursor.chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect());

        Some(Self { heart_rate, energy_expended, rr_intervals })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let adp = bluest::Adapter::default().await.ok_or("no adapter found")?;
    adp.wait_available().await?;

    print!("Starting OSC service...");
    let vrc = vrchat_osc::VRChatOSC::new(None).await?;
    println!("OK!");

    print!("Looking for device... ");
    let devs = adp
        .connected_devices_with_services(&[bluest::btuuid::services::HEART_RATE])
        .await?;
    let dev = devs
        .first()
        .ok_or("no devices found!")?;
    println!("OK!");

    print!("Discovering services... ");
    let svs = dev
        .discover_services_with_uuid(bluest::btuuid::services::HEART_RATE)
        .await?;
    let sv = svs
        .first()
        .ok_or("device does not have 'HEART_RATE' service")?;
    println!("OK!");

    print!("Discovering characteristics... ");
    let cxs = sv
        .discover_characteristics_with_uuid(bluest::btuuid::characteristics::HEART_RATE_MEASUREMENT)
        .await?;
    let cx = cxs
        .first()
        .ok_or("device does not have 'HEART_RATE_MEASUREMENT' characteristic")?;
    println!("OK!");

    print!("Subscribing to GATT notifications... ");
    let mut stream = cx
        .notify()
        .await?;
    println!("OK!");

    while let Some(event) = stream.next().await {
        if let Some(m) = HeartRateMeasurement::from_bytes(&event?) {
            use vrchat_osc::rosc::*;
            use std::{convert::TryFrom, time::UNIX_EPOCH};

            println!("bpm: {:4?}", m.heart_rate);
            let time = OscTime::try_from(UNIX_EPOCH).unwrap();
            let packets = {
                let hr = m.heart_rate as i32;
                use OscType as v;
                [
                    ("HR"        , v::Int( hr            )),
                    ("onesHR"    , v::Int( hr        % 10)),
                    ("tensHR"    , v::Int((hr / 10  )% 10)),
                    ("hundredsHR", v::Int((hr / 100 )% 10)),
                    ("floatHR"   , v::Float((hr as f32) * 0.0078125 - 1.0)),
                ]
            }.map(|(s, v)| OscPacket::Message(OscMessage {
                addr: "/avatar/parameters/".to_string() + s,
                args: vec![v],
            })).to_vec();

            let b = OscPacket::Bundle(OscBundle{
                timetag: time,
                content: packets,
            });
            vrc.send(b, "VRChat-Client-*").await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hrm_parsing() {
        let data = [0b11001, 0x4B, 0x00, 0x32, 0x00, 0xE8, 0x03, 0x20, 0x03];
        let result = HeartRateMeasurement::from_bytes(&data).unwrap();

        assert_eq!(result.heart_rate, 0x4B);
        assert_eq!(result.energy_expended, Some(0x32));
        assert_eq!(result.rr_intervals, Some(vec![0x03E8, 0x0320]));
    }
}
