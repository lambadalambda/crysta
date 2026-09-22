//! Source-derived Crysta startup through physical APU ports, never RAM injection.
use crate::music_data::{MusicData, TransferGroup};
use spc_player::Apu;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Boot and start selection3. RAM reads only verify source uploads; they never
/// initialize state. Every driver instruction runs on the audio-only SPC.
pub fn initialize(data: &MusicData) -> Result<Apu> {
    let mut apu = Apu::new()?;
    upload(&mut apu, &data.bootstrap)?;
    apu.run_cycles(20_000)?;
    apu.write_port(0, 0)?;
    apu.write_port(1, data.stop_parameter)?;
    apu.write_port(0, 0xf0)?;
    apu.run_cycles(18_000)?;
    wait(&mut apu, 0, 0)?;
    apu.run_cycles(36_000)?;
    apu.write_port(0, 0xff)?;
    apu.run_cycles(36_000)?;
    upload(&mut apu, &data.sequence)?;
    apu.write_port(0, 0xff)?;
    upload(&mut apu, &data.samples)?;
    // Catch dropped/corrupted transfers before playing. In particular the
    // resident receiver echoes ACK before reading the payload input latch.
    for group in [&data.sequence, &data.samples] {
        for block in &group.blocks {
            let mut actual = vec![0; block.data.len()];
            apu.read_ram(usize::from(block.destination), &mut actual)?;
            if actual != block.data {
                return Err(format!("music upload mismatch at ${:04x}", block.destination).into());
            }
        }
    }
    for port in 1..=3 {
        apu.write_port(port, 0)?;
    }
    apu.run_cycles(54_000)?;
    apu.write_port(0, 0xf4)?;
    Ok(apu)
}

fn wait(apu: &mut Apu, port: usize, value: u8) -> Result<()> {
    for _ in 0..4096 {
        if apu.read_port(port)? == value {
            return Ok(());
        }
        apu.run_cycles(16)?;
    }
    Err(format!("music upload timed out waiting for port{port}=${value:02x}").into())
}

fn next_token(length: usize) -> u8 {
    let token = length.wrapping_add(3).to_le_bytes()[0];
    if token == 0 {
        4
    } else {
        token
    }
}

fn upload(apu: &mut Apu, group: &TransferGroup) -> Result<()> {
    if group.blocks.len() > 16
        || group.blocks.iter().any(|block| {
            block.data.is_empty() || usize::from(block.destination) + block.data.len() > 0x10000
        })
    {
        return Err("invalid bounded music transfer".into());
    }
    wait(apu, 0, 0xaa)?;
    wait(apu, 1, 0xbb)?;
    let mut token = 0xcc;
    let blocks = group
        .blocks
        .iter()
        .map(|block| (block.destination, block.data.as_slice()));
    for (destination, payload) in
        blocks.chain(std::iter::once((group.terminal_destination, &[][..])))
    {
        let [low, high] = destination.to_le_bytes();
        apu.write_port(2, low)?;
        apu.write_port(3, high)?;
        apu.write_port(1, u8::from(!payload.is_empty()))?;
        apu.write_port(0, token)?;
        wait(apu, 0, token)?;
        for (index, &value) in payload.iter().enumerate() {
            apu.write_port(1, value)?;
            let counter = index.to_le_bytes()[0];
            apu.write_port(0, counter)?;
            wait(apu, 0, counter)?;
            // SPC $0421 echoes before $0423 reads port1. Do not overwrite it.
            apu.run_cycles(32)?;
        }
        token = next_token(payload.len());
    }
    apu.write_port(2, 0)?;
    apu.write_port(3, 0)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music_data::{extract_crysta_music, Transfer};

    #[test]
    fn header_tokens_skip_zero_and_wrap_like_the_host() {
        assert_eq!(next_token(1), 4);
        assert_eq!(next_token(253), 4);
        assert_eq!(next_token(254), 1);
        assert_eq!(next_token(65535), 2);
    }

    #[test]
    fn uploads_synthetic_program_through_physical_ipl() {
        let code = vec![0x8f, 0x42, 0xf6, 0x2f, 0xfe]; // MOV $F6,#$42; BRA self
        let group = TransferGroup {
            blocks: vec![Transfer {
                destination: 0x300,
                source_ranges: vec![],
                data: code.clone(),
            }],
            terminal_destination: 0x300,
        };
        let mut apu = Apu::new().unwrap();
        upload(&mut apu, &group).unwrap();
        apu.run_cycles(128).unwrap();
        assert_eq!(apu.read_port(2).unwrap(), 0x42);
        let mut actual = vec![0; code.len()];
        apu.read_ram(0x300, &mut actual).unwrap();
        assert_eq!(actual, code);
    }

    #[test]
    fn missing_ack_times_out_instead_of_hanging() {
        let mut apu = Apu::new().unwrap();
        assert!(wait(&mut apu, 0, 0x12).is_err());
    }

    #[test]
    #[ignore = "requires owned JP ROM: set CRYSTA_JP_ROM"]
    fn source_only_crysta_is_non_silent_and_chunk_deterministic() {
        let bytes = std::fs::read(std::env::var("CRYSTA_JP_ROM").unwrap()).unwrap();
        let rom = rom::Rom::load(&bytes).unwrap();
        let data = extract_crysta_music(&rom).unwrap();
        let mut first = initialize(&data).unwrap(); // verifies upload RAM before start
        let mut second = initialize(&data).unwrap();
        let mut expected = vec![0; 64_000];
        first.render(&mut expected).unwrap();
        let mut actual = vec![0; expected.len()];
        for chunk in actual.chunks_mut(254) {
            second.render(chunk).unwrap();
        }
        assert_eq!(actual, expected);
        assert!(actual.iter().any(|&sample| sample != 0));
        // Keep running past ring and 16-bit counter wrap, without further host commands.
        for _ in 0..9 {
            first.render(&mut expected).unwrap();
        }
        assert!(expected.iter().filter(|&&s| s != 0).count() > 32_000);
    }
}
