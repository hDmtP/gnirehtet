use byteorder::{BigEndian, ByteOrder};
use std::mem;

pub struct IPv4Header<'a> {
    raw: &'a mut [u8],
    data: &'a mut IPv4HeaderData,
}

#[derive(Clone)]
pub struct IPv4HeaderData {
    version: u8,
    header_length: u8,
    total_length: u16,
    protocol: Protocol,
    source: u32,
    destination: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Protocol {
    TCP,
    UDP,
    OTHER,
}

impl IPv4HeaderData {
    pub fn parse(raw: &[u8]) -> Self {
        Self {
            version: raw[0] >> 4,
            header_length: (raw[0] & 0xf) << 2,
            total_length: BigEndian::read_u16(&raw[2..4]),
            protocol: match raw[9] {
                6 => Protocol::TCP,
                17 => Protocol::UDP,
                _ => Protocol::OTHER
            },
            source: BigEndian::read_u32(&raw[12..16]),
            destination: BigEndian::read_u32(&raw[16..20]),
        }
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn header_length(&self) -> u8 {
        self.header_length
    }

    pub fn total_length(&self) -> u16 {
        self.total_length
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    pub fn source(&self) -> u32 {
        self.source
    }

    pub fn destination(&self) -> u32 {
        self.destination
    }
}

impl<'a> IPv4Header<'a> {
    pub fn new(raw: &'a mut [u8], data: &'a mut IPv4HeaderData) -> Self {
        Self {
            raw: raw,
            data: data,
        }
    }

    pub fn raw(&self) -> &[u8] {
        self.raw
    }

    pub fn raw_mut(&mut self) -> &mut [u8] {
        self.raw
    }

    pub fn data(&self) -> &IPv4HeaderData {
        self.data
    }

    pub fn data_mut(&mut self) -> &mut IPv4HeaderData {
        self.data
    }

    pub fn header_length(&self) -> u8 {
        self.data.header_length
    }

    pub fn total_length(&self) -> u16 {
        self.data.total_length
    }

    pub fn protocol(&self) -> Protocol {
        self.data.protocol
    }

    pub fn source(&self) -> u32 {
        self.data.source
    }

    pub fn destination(&self) -> u32 {
        self.data.destination
    }

    pub fn set_total_length(&mut self, total_length: u16) {
        self.data.total_length = total_length;
        BigEndian::write_u16(&mut self.raw[2..4], total_length);
    }

    pub fn set_source(&mut self, source: u32) {
        self.data.source = source;
        BigEndian::write_u32(&mut self.raw[12..16], source);
    }

    pub fn set_destination(&mut self, destination: u32) {
        self.data.destination = destination;
        BigEndian::write_u32(&mut self.raw[16..20], destination);
    }

    pub fn swap_source_and_destination(&mut self) {
        mem::swap(&mut self.data.source, &mut self.data.destination);
        for i in 12..16 {
            self.raw.swap(i, i + 4);
        }
    }

    pub fn compute_checksum(&mut self) {
        // reset checksum field
        self.set_checksum(0);

        let j = self.data.header_length as usize / 2;
        let mut sum = (0..j).map(|i| {
            let range = 2*i..2*(i+1);
            BigEndian::read_u16(&self.raw[range]) as u32
        }).sum::<u32>();
        while (sum & !0xffff) != 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }

        self.set_checksum(!sum as u16);
    }

    fn checksum(&self) -> u16 {
        BigEndian::read_u16(&self.raw[10..12])
    }

    fn set_checksum(&mut self, checksum: u16) {
        BigEndian::write_u16(&mut self.raw[10..12], checksum);
    }

    pub fn read_version(raw: &[u8]) -> Option<u8> {
        if raw.is_empty() {
            None
        } else {
            // version is stored in the 4 first bits
            Some(raw[0] >> 4)
        }
    }

    pub fn read_length(raw: &[u8]) -> Option<u16> {
        if raw.len() < 4 {
            None
        } else {
            // packet length is 16 bits starting at offset 2
            let length = BigEndian::read_u16(&raw[2..4]);
            Some(length)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{BigEndian, WriteBytesExt};

    fn create_header() -> Vec<u8> {
        let mut raw: Vec<u8> = Vec::new();
        raw.reserve(20);
        raw.write_u8(4u8 << 4 | 5).unwrap(); // version_and_ihl
        raw.write_u8(0).unwrap(); //ToS
        raw.write_u16::<BigEndian>(28).unwrap(); // total length
        raw.write_u32::<BigEndian>(0).unwrap(); // id_flags_fragment_offset
        raw.write_u8(0).unwrap(); // TTL
        raw.write_u8(17).unwrap(); // protocol (UDP)
        raw.write_u16::<BigEndian>(0).unwrap(); // checksum
        raw.write_u32::<BigEndian>(0x12345678).unwrap(); // source address
        raw.write_u32::<BigEndian>(0x42424242).unwrap(); // destination address
        raw
    }

    #[test]
    fn parse_header() {
        let raw = &create_header()[..];
        let data = IPv4HeaderData::parse(raw);
        assert_eq!(4, data.version);
        assert_eq!(20, data.header_length);
        assert_eq!(28, data.total_length);
        assert_eq!(Protocol::UDP, data.protocol);
        assert_eq!(0x12345678, data.source);
        assert_eq!(0x42424242, data.destination);
    }

    #[test]
    fn edit_header() {
        let raw = &mut create_header()[..];
        let mut data = IPv4HeaderData::parse(raw);
        let mut header = IPv4Header::new(raw, data);

        header.set_source(0x87654321);
        header.set_destination(0x24242424);
        header.set_total_length(42);
        assert_eq!(0x87654321, header.source());
        assert_eq!(0x24242424, header.destination());
        assert_eq!(42, header.total_length());

        // assert that the buffer has been modified
        let raw_source = BigEndian::read_u32(&header.raw[12..16]);
        let raw_destination = BigEndian::read_u32(&header.raw[16..20]);
        let raw_total_length = BigEndian::read_u16(&header.raw[2..4]);
        assert_eq!(0x87654321, raw_source);
        assert_eq!(0x24242424, raw_destination);
        assert_eq!(42, raw_total_length);

        header.swap_source_and_destination();

        assert_eq!(0x24242424, header.source());
        assert_eq!(0x87654321, header.destination());

        let raw_source = BigEndian::read_u32(&header.raw[12..16]);
        let raw_destination = BigEndian::read_u32(&header.raw[16..20]);
        assert_eq!(0x24242424, raw_source);
        assert_eq!(0x87654321, raw_destination);
    }

    #[test]
    fn compute_checksum() {
        let raw = &mut create_header()[..];
        let mut data = IPv4HeaderData::parse(raw);
        let mut header = IPv4Header::new(raw, data);

        // set a fake checksum value to assert that it is correctly computed
        header.set_checksum(0x79);

        header.compute_checksum();

        let mut sum: u32 = 0x4500 + 0x001C + 0x0000 + 0x0000 + 0x0011 +
                           0x0000 + 0x1234 + 0x5678 + 0x4242 + 0x4242;
        while (sum & !0xffff) != 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }
        let sum = !sum as u16;
        assert_eq!(sum, header.checksum());
    }

    #[test]
    fn read_ip_version_unavailable() {
        let empty_slice = &[][..0];
        let version = IPv4Header::read_version(empty_slice);
        assert!(version.is_none());
    }

    #[test]
    fn read_ip_version_available() {
        let version_and_ihl: u8 = (4 << 4) | 5;
        let raw = [ version_and_ihl, 0, 0, 0, 0, 0, 0, 0 ];
        let version = IPv4Header::read_version(&raw);
        assert_eq!(4, version.unwrap());
    }

    #[test]
    fn read_ip_length_unavailable() {
        let empty_slice = &[][..0];
        let length = IPv4Header::read_length(empty_slice);
        assert!(length.is_none());
    }

    #[test]
    fn read_ip_length_available() {
        let raw = [ 0u8, 0, 0x01, 0x23 ];
        let length = IPv4Header::read_length(&raw);
        assert_eq!(0x123, length.unwrap());
    }
}
