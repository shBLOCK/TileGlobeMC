use pymeta::pymeta;
use crate::world::block::blocks::GenericBlock;
use crate::world::block::blocks::lever::LeverBlock;
use crate::world::block::blocks::redstone_block::RedstoneBlock;
use crate::world::block::blocks::redstone_comparator::RedstoneComparatorBlock;
use crate::world::block::blocks::redstone_repeater::RedstoneRepeaterBlock;
use crate::world::block::blocks::redstone_wire::RedstoneWireBlock;
use crate::world::block::{Block, DynifiedBlock, StateIdType};
use crate::world::block::{BlockState, BlockStateType, StateId};
use tileglobe_utils::resloc::ResLoc;
use crate::world::block::blocks::redstone_lamp::RedstoneLampBlock;
use crate::world::block::blocks::redstone_torch::{RedstoneTorchBlock, RedstoneTorchType};
use crate::world::utils::mc_block_id_base;

pub struct Blocks;
impl Blocks {
    const _ID_BASE_TO_BLOCK_SORTED: &[(BlockStateType, &'static dyn DynifiedBlock)] = &pymeta! {${
        from tg_utils import *;

        ENTRIES = {
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
                    resloc: BlockResLocs::IRON_BLOCK,
                    id_base: mc_block_id_base!("iron_block"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "white_wool": {{
                &_TmpBlock {
                    resloc: BlockResLocs::WHITE_WOOL,
                    id_base: mc_block_id_base!("white_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "red_wool": {{
                &_TmpBlock {
                    resloc: BlockResLocs::RED_WOOL,
                    id_base: mc_block_id_base!("red_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "orange_wool": {{
                &_TmpBlock {
                    resloc: BlockResLocs::ORANGE_WOOL,
                    id_base: mc_block_id_base!("orange_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "yellow_wool": {{
                &_TmpBlock {
                    resloc: BlockResLocs::YELLOW_WOOL,
                    id_base: mc_block_id_base!("yellow_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "green_wool": {{
                &_TmpBlock {
                    resloc: BlockResLocs::GREEN_WOOL,
                    id_base: mc_block_id_base!("green_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "blue_wool": {{
                &_TmpBlock {
                    resloc: BlockResLocs::BLUE_WOOL,
                    id_base: mc_block_id_base!("blue_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "black_wool": {{
                &_TmpBlock {
                    resloc: BlockResLocs::BLACK_WOOL,
                    id_base: mc_block_id_base!("black_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
            "magenta_wool": {{
                &_TmpBlock {
                    resloc: BlockResLocs::MAGENTA_WOOL,
                    id_base: mc_block_id_base!("magenta_wool"),
                    num_states: 1,
                    default_state: StateId(0),
                    is_redstone_conductor: true,
                }
            }},
        };
        ENTRIES = {ResLoc(k): v for k, v in ENTRIES.items()};
        
        elements = [];
        for block in sorted(BlockDef.load_all(), key = lambda it: it.id_base):{
            if block.resloc in ENTRIES:{
                element = ENTRIES[block.resloc];
            } else:{
                element = {{
                    &GenericBlock {
                        resloc: BlockResLocs::$block.resloc.as_ident().upper()$,
                        id_base: $block.id_base$,
                        num_states: $block.total_states$,
                        default_state: StateId($block.default_state$),
                    }
                }};
            }
            elements.append({{ ($block.id_base$, $element$) }});
        }

        emit({{ [$Punct(',').join(elements)$] }});
    }};

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
    pymeta! {
        $from tg_utils.block_def import BlockDef;
        $for resloc, _ in BlockDef.all_reslocs():{
            pub const $resloc.as_ident().upper()$: &'static ResLoc<'static> = &ResLoc::new(
                ${{ tileglobe_utils::MINECRAFT }} if resloc.is_minecraft() else lit(resloc.namespace)$,
                $lit(resloc.path)$
            );
        }
    }
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
