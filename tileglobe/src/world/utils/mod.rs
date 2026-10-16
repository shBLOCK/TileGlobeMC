use pymeta::pymeta_func;

#[pymeta_func(resloc: str)]
#[public]
mc_block_id_base! {
    from tg_utils import ResLoc, BlockDef;
    return BlockDef.load(ResLoc(resloc)).id_base;
}