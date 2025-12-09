use crate::world::block::blocks::GenericBlock;
use crate::world::block::blocks::lever::LeverBlock;
use crate::world::block::blocks::redstone_block::RedstoneBlock;
use crate::world::block::blocks::redstone_comparator::RedstoneComparatorBlock;
use crate::world::block::blocks::redstone_repeater::RedstoneRepeaterBlock;
use crate::world::block::blocks::redstone_wire::RedstoneWireBlock;
use crate::world::block::{Block, DynifiedBlock, StateIdType};
use crate::world::block::{BlockState, BlockStateType, StateId};
use tileglobe_proc_macro::mc_block_id_base;
use tileglobe_utils::resloc::ResLoc;
use crate::world::block::blocks::redstone_lamp::RedstoneLampBlock;
use crate::world::block::blocks::redstone_torch::{RedstoneTorchBlock, RedstoneTorchType};

pub struct Blocks;
impl Blocks {
    const _ID_BASE_TO_BLOCK_SORTED: &[(BlockStateType, &'static dyn DynifiedBlock)] = &tileglobe_proc_macro::mc_blocks_registry! {
        generic_block: GenericBlock,
        resloc_consts: BlockResLocs,
        entries: {
            "lever" => &LeverBlock,
            "repeater" => &RedstoneRepeaterBlock,
            "comparator" => &RedstoneComparatorBlock,
            "redstone_wire" => &RedstoneWireBlock,
            "redstone_block" => &RedstoneBlock,
            "redstone_torch" => &RedstoneTorchBlock { torch_type: RedstoneTorchType::Floor },
            "redstone_wall_torch" => &RedstoneTorchBlock { torch_type: RedstoneTorchType::Wall },
            "redstone_lamp" => &RedstoneLampBlock,
            "iron_block" => &_TmpBlock {
                resloc: BlockResLocs::IRON_BLOCK,
                id_base: mc_block_id_base!("iron_block"),
                num_states: 1,
                default_state: StateId(0),
                is_redstone_conductor: true,
            },
            "white_wool" => &_TmpBlock {
                resloc: BlockResLocs::WHITE_WOOL,
                id_base: mc_block_id_base!("white_wool"),
                num_states: 1,
                default_state: StateId(0),
                is_redstone_conductor: true,
            },
            "red_wool" => &_TmpBlock {
                resloc: BlockResLocs::RED_WOOL,
                id_base: mc_block_id_base!("red_wool"),
                num_states: 1,
                default_state: StateId(0),
                is_redstone_conductor: true,
            },
            "orange_wool" => &_TmpBlock {
                resloc: BlockResLocs::ORANGE_WOOL,
                id_base: mc_block_id_base!("orange_wool"),
                num_states: 1,
                default_state: StateId(0),
                is_redstone_conductor: true,
            },
            "yellow_wool" => &_TmpBlock {
                resloc: BlockResLocs::YELLOW_WOOL,
                id_base: mc_block_id_base!("yellow_wool"),
                num_states: 1,
                default_state: StateId(0),
                is_redstone_conductor: true,
            },
            "green_wool" => &_TmpBlock {
                resloc: BlockResLocs::GREEN_WOOL,
                id_base: mc_block_id_base!("green_wool"),
                num_states: 1,
                default_state: StateId(0),
                is_redstone_conductor: true,
            },
            "blue_wool" => &_TmpBlock {
                resloc: BlockResLocs::BLUE_WOOL,
                id_base: mc_block_id_base!("blue_wool"),
                num_states: 1,
                default_state: StateId(0),
                is_redstone_conductor: true,
            },
            "black_wool" => &_TmpBlock {
                resloc: BlockResLocs::BLACK_WOOL,
                id_base: mc_block_id_base!("black_wool"),
                num_states: 1,
                default_state: StateId(0),
                is_redstone_conductor: true,
            },
            "magenta_wool" => &_TmpBlock {
                resloc: BlockResLocs::MAGENTA_WOOL,
                id_base: mc_block_id_base!("magenta_wool"),
                num_states: 1,
                default_state: StateId(0),
                is_redstone_conductor: true,
            },
        }
    };

    pub(super) fn get_block(&self, bs: BlockState) -> &'static dyn DynifiedBlock {
        match Self::_ID_BASE_TO_BLOCK_SORTED.binary_search_by_key(&bs.0, |&(id, _)| id) {
            Ok(idx) => Self::_ID_BASE_TO_BLOCK_SORTED[idx].1,
            Err(0) => unreachable!(),
            Err(idx) => Self::_ID_BASE_TO_BLOCK_SORTED[idx - 1].1,
        }
    }
}

pub struct BlockResLocs;
impl BlockResLocs {
    tileglobe_proc_macro::mc_block_resloc_consts!();
}

#[derive(Debug)]
pub struct _TmpBlock {
    pub resloc: &'static ResLoc<'static>,
    pub id_base: BlockStateType,
    pub num_states: StateIdType,
    pub default_state: StateId,
    pub is_redstone_conductor: bool,
}

impl Block for _TmpBlock {
    fn resloc(&self) -> &'static ResLoc<'static> {
        self.resloc
    }

    fn default_state(&self) -> BlockState {
        BlockState(self.id_base + self.default_state.0)
    }

    fn is_redstone_conductor(&self, blockstate: BlockState) -> bool {
        self.is_redstone_conductor
    }
}
