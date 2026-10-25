package dev.shblock.tileglobemc.datagen

import com.google.gson.JsonObject
import net.minecraft.core.HolderLookup
import net.minecraft.core.registries.Registries
import net.minecraft.data.CachedOutput
import net.minecraft.data.DataProvider
import net.minecraft.data.PackOutput
import net.minecraft.world.item.BlockItem
import java.util.concurrent.CompletableFuture

class TmpBlockItemDatagen(
    val packOutput: PackOutput,
    val registries: CompletableFuture<HolderLookup.Provider>
) : DataProvider {
    override fun run(cachedOutput: CachedOutput) = registries.thenCompose { registries ->
        val output = JsonObject()
        registries.lookupOrThrow(Registries.ITEM).listElements().toList().mapNotNull { entry ->
            val item = entry.value() as? BlockItem ?: return@mapNotNull null
            val rid = entry.key().identifier()
            val blockRid = item.block.properties().blockIdOrThrow().identifier()
            output.addProperty(rid.toString(), blockRid.toString())
        }
        DataProvider.saveStable(
            cachedOutput,
            output,
            packOutput.outputFolder.resolve("tmp_block_items.json")
        )
    }

    override fun getName() = "TileGlobeMC: TmpBlockItem"
}