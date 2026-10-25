use crate::utils::MINECRAFT;
use crate::utils::RID;
use crate::world::block::blocks::GenericBlock;
use crate::world::block::blocks::lever::LeverBlock;
use crate::world::block::blocks::redstone_block::RedstoneBlock;
use crate::world::block::blocks::redstone_comparator::RedstoneComparatorBlock;
use crate::world::block::blocks::redstone_lamp::RedstoneLampBlock;
use crate::world::block::blocks::redstone_repeater::RedstoneRepeaterBlock;
use crate::world::block::blocks::redstone_torch::{RedstoneTorchBlock, RedstoneTorchType};
use crate::world::block::blocks::redstone_wire::RedstoneWireBlock;
use crate::world::block::{Block, DynifiedBlock, StateIdType};
use crate::world::block::{BlockState, BlockStateType, StateId};
use crate::world::utils::mc_block_index_base;
use pymeta::pymeta;

pub struct Blocks;
impl Blocks {
    const _INDEX_BASE_TO_BLOCK_SORTED: &[(BlockStateType, &'static dyn DynifiedBlock)] = &pymeta! {${
        from tg_utils import *;

        BLOCKS = {
            "lever": {{ &LeverBlock }},
            "repeater": {{ &RedstoneRepeaterBlock }},
            "comparator": {{ &RedstoneComparatorBlock }},
            "redstone_wire": {{ &RedstoneWireBlock }},
            "redstone_block": {{ &RedstoneBlock }},
            "redstone_torch": {{ &RedstoneTorchBlock { torch_type: RedstoneTorchType::Floor } }},
            "redstone_wall_torch": {{ &RedstoneTorchBlock { torch_type: RedstoneTorchType::Wall } }},
            "redstone_lamp": {{ &RedstoneLampBlock }},
            "iron_block": {{
                &_TmpBlock {
                    rid: BlockRIDs::IRON_BLOCK,
                    index_base: mc_block_index_base!("iron_block"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "white_wool": {{
                &_TmpBlock {
                    rid: BlockRIDs::WHITE_WOOL,
                    index_base: mc_block_index_base!("white_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "red_wool": {{
                &_TmpBlock {
                    rid: BlockRIDs::RED_WOOL,
                    index_base: mc_block_index_base!("red_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "orange_wool": {{
                &_TmpBlock {
                    rid: BlockRIDs::ORANGE_WOOL,
                    index_base: mc_block_index_base!("orange_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "yellow_wool": {{
                &_TmpBlock {
                    rid: BlockRIDs::YELLOW_WOOL,
                    index_base: mc_block_index_base!("yellow_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "green_wool": {{
                &_TmpBlock {
                    rid: BlockRIDs::GREEN_WOOL,
                    index_base: mc_block_index_base!("green_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "blue_wool": {{
                &_TmpBlock {
                    rid: BlockRIDs::BLUE_WOOL,
                    index_base: mc_block_index_base!("blue_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "black_wool": {{
                &_TmpBlock {
                    rid: BlockRIDs::BLACK_WOOL,
                    index_base: mc_block_index_base!("black_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "magenta_wool": {{
                &_TmpBlock {
                    rid: BlockRIDs::MAGENTA_WOOL,
                    index_base: mc_block_index_base!("magenta_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
        };
        BLOCKS = {RID(k): v for k, v in BLOCKS.items()};

        entries = [];
        for block in sorted(BlockDef.load_all(), key = lambda it: it.index_base):{
            if block.rid in BLOCKS:{
                entry = BLOCKS[block.rid];
            } else:{
                entry = {{
                    &GenericBlock {
                        rid: BlockRIDs::$block.rid.as_ident().upper()$,
                        index_base: $block.index_base$,
                        num_states: $block.total_states$,
                        default_state: StateId($block.default_state$),
                    }
                }};
            }
            entries.append({{ ($block.index_base$, $entry$) }});
        }

        emit({{ [$Punct(',').join(entries)$] }});
    }};

    pub(super) fn get_block(&self, bs: BlockState) -> &'static dyn DynifiedBlock {
        match Self::_INDEX_BASE_TO_BLOCK_SORTED.binary_search_by_key(&bs.0, |&(id, _)| id) {
            Ok(idx) => Self::_INDEX_BASE_TO_BLOCK_SORTED[idx].1,
            Err(0) => unreachable!(),
            Err(idx) => Self::_INDEX_BASE_TO_BLOCK_SORTED[idx - 1].1,
        }
    }
}

pub struct BlockRIDs;
impl BlockRIDs {
    pymeta! {
        $from tg_utils.block_def import BlockDef;
        $for rid, _ in BlockDef.all_rids():{
            pub const $rid.as_ident().upper()$: &'static RID<'static> = &RID::new(
                ${{ MINECRAFT }} if rid.is_minecraft() else lit(rid.namespace)$,
                $lit(rid.path)$
            );
        }
    }
}

#[derive(Debug)]
pub struct _TmpBlock {
    pub rid: &'static RID<'static>,
    pub index_base: BlockStateType,
    pub num_states: StateIdType,
    pub default_state: StateId,
    pub is_redstone_conductor: bool,
}

impl Block for _TmpBlock {
    fn rid(&self) -> &'static RID<'static> {
        self.rid
    }

    fn default_state(&self) -> BlockState {
        BlockState(self.index_base + self.default_state.0)
    }

    fn is_redstone_conductor(&self, blockstate: BlockState) -> bool {
        self.is_redstone_conductor
    }
}
