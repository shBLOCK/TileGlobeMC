package dev.shblock.tileglobemc

import dev.shblock.tileglobemc.datagen.BlockDefDatagen
import dev.shblock.tileglobemc.datagen.TmpBlockItemDatagen
import net.fabricmc.fabric.api.datagen.v1.DataGeneratorEntrypoint
import net.fabricmc.fabric.api.datagen.v1.FabricDataGenerator
import java.nio.file.Path

object TileGlobeMCDataGenerator : DataGeneratorEntrypoint {
    override fun onInitializeDataGenerator(fabricDataGenerator: FabricDataGenerator) {
        fabricDataGenerator.createPack().apply {
            addProvider(::BlockDefDatagen)
            addProvider(::TmpBlockItemDatagen)
        }
    }
}