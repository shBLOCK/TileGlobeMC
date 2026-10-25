use pymeta::pymeta_func;

#[pymeta_func(rid: str)]
#[public]
mc_block_index_base! {
    from tg_utils import RID, BlockDef;
    return BlockDef.load(RID(rid)).index_base;
}
