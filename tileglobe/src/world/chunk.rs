use crate::world::block::BlockState;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use core::ops::RangeInclusive;
use tileglobe_utils::network::{EIOError, MCPacketBuffer, WriteNumPrimitive, WriteVarInt};
use tileglobe_utils::pos::{ChunkLocalPos, ChunkPos};

pub struct Chunk {
    sections: Vec<ChunkSection>,
    bottom_section: i8,
    //TODO: entities
}

impl Chunk {
    pub fn new(sections: RangeInclusive<i8>) -> Self {
        let bottom = *sections.start();
        Self {
            sections: sections.map(|y| ChunkSection::new(y)).collect(),
            bottom_section: bottom,
        }
    }

    pub fn get_section(&self, y: i8) -> Result<&ChunkSection, ()> {
        self.sections
            .get((y - self.bottom_section) as usize)
            .ok_or(())
    }

    pub fn get_section_mut(&mut self, y: i8) -> Result<&mut ChunkSection, ()> {
        self.sections
            .get_mut((y - self.bottom_section) as usize)
            .ok_or(())
    }

    pub fn get_block_state(&self, pos: ChunkLocalPos) -> Result<BlockState, ()> {
        Ok(self
            .get_section(pos.section())?
            .get_block_state(pos.section_block_index()))
    }

    pub fn set_block_state(
        &mut self,
        pos: ChunkLocalPos,
        bs: BlockState,
    ) -> Result<BlockState, ()> {
        Ok(self
            .get_section_mut(pos.section())?
            .set_block_state(pos.section_block_index(), bs))
    }

    pub async fn gen_blocks_update_packets_and_clear_changes(
        &mut self,
        chunk_pos: ChunkPos,
    ) -> Vec<MCPacketBuffer> {
        let mut vec = Vec::<MCPacketBuffer>::with_capacity(self.sections.len());
        for section in self.sections.iter_mut() {
            if let Some(pkt) = section
                .gen_blocks_update_packet_and_clear_changes(chunk_pos)
                .await
            {
                vec.push(pkt);
            }
        }
        vec
    }
}

pub struct ChunkSection {
    section_y: i8,
    blocks: [BlockState; 16 * 16 * 16],
    non_air_blocks: u16,
    changes: BTreeSet<u16>,
}

impl ChunkSection {
    pub fn new(section_y: i8) -> Self {
        Self {
            section_y,
            blocks: [Default::default(); 16 * 16 * 16],
            non_air_blocks: 0,
            changes: BTreeSet::new(),
        }
    }

    pub fn get_block_state(&self, index: u16) -> BlockState {
        self.blocks[index as usize]
    }

    pub fn set_block_state(&mut self, index: u16, blockstate: BlockState) -> BlockState {
        let old = self.blocks[index as usize];
        self.blocks[index as usize] = blockstate;
        if blockstate != old {
            self.changes.insert(index);
        }
        if old.is_air() {
            if !blockstate.is_air() {
                self.non_air_blocks += 1;
            }
        } else {
            if blockstate.is_air() {
                self.non_air_blocks -= 1;
            }
        }
        old
    }

    pub fn get_data_array(&self) -> &[BlockState; 16 * 16 * 16] {
        &self.blocks
    }

    pub fn non_air_blocks(&self) -> u16 {
        self.non_air_blocks
    }

    pub fn serialized_size(&self) -> usize {
        2 + if self.non_air_blocks() == 0 {
            1 + 1
        } else {
            1 + (16usize * 16 * 16).div_ceil(64 / 15) * 8
        }
    }

    pub async fn serialize_into<W: embedded_io_async::Write>(
        &self,
        writer: &mut W,
    ) -> Result<(), EIOError<W::Error>> {
        let non_air_blocks = self.non_air_blocks();
        if non_air_blocks == 0 {
            writer.write_be(0u16).await?;
            writer.write_be(0u8).await?;
            writer.write_varint(0u32).await?;
        } else {
            writer.write_be::<u16>(non_air_blocks).await?;
            let entry_size = 15u8;
            writer.write_be(entry_size).await?;
            let entries_per_long = 64u8 / entry_size;
            let mut data = self.get_data_array().as_slice();
            let mut i = 0;
            let mut long = 0u64;
            while !data.is_empty() {
                let n = (i % entries_per_long as u16) as u8;
                long |= (data[0].0 as u64) << (n * entry_size);
                data = &data[1..];
                if n == (entries_per_long - 1) || data.is_empty() {
                    writer.write_be(long).await?;
                    long = 0;
                }
                i += 1;
            }
        }
        Ok(())
    }

    async fn gen_blocks_update_packet_and_clear_changes(
        &mut self,
        chunk_pos: ChunkPos,
    ) -> Option<MCPacketBuffer> {
        if self.changes.is_empty() {
            return None;
        }
        let mut pkt = MCPacketBuffer::new(77).await; // section_blocks_update
        pkt.write_be::<u64>(
            (self.section_y as u64 & 0xFFFFF)
                | ((chunk_pos.y as u64 & 0x3FFFFF) << 20)
                | ((chunk_pos.x as u64 & 0x3FFFFF) << 42),
        )
        .await
        .unwrap();
        pkt.write_varint(self.changes.len() as u32).await.unwrap();
        for &pos in self.changes.iter() {
            let blockstate = self.get_block_state(pos);
            let encoded_pos = (pos & 0xF) << 8 | ((pos >> 8) & 0xF) | (pos & 0xF0);
            let element = ((blockstate.0 as u64) << 12) | (encoded_pos as u64);
            pkt.write_varint::<u64>(element)
                .await
                .unwrap();
        }
        self.changes.clear();
        Some(pkt)
    }
}
