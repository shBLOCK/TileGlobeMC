package dev.shblock.tileglobemc.datagen

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import net.minecraft.core.HolderLookup
import net.minecraft.core.registries.Registries
import net.minecraft.data.CachedOutput
import net.minecraft.data.DataProvider
import net.minecraft.data.PackOutput
import net.minecraft.world.level.block.Block
import net.minecraft.world.level.block.state.BlockState
import net.minecraft.world.level.block.state.properties.BooleanProperty
import net.minecraft.world.level.block.state.properties.EnumProperty
import net.minecraft.world.level.block.state.properties.IntegerProperty
import java.util.concurrent.CompletableFuture

private val BlockState.id get() = Block.getId(this)

class BlockDefDatagen(
    val packOutput: PackOutput,
    val registries: CompletableFuture<HolderLookup.Provider>
) : DataProvider {
    override fun run(cachedOutput: CachedOutput) = registries.thenCompose { registries ->
        CompletableFuture.allOf(
            *registries.lookupOrThrow(Registries.BLOCK).listElements().map { entry ->
                val resLoc = entry.key().location()
                val block = entry.value()

                val blockData = JsonObject()

                blockData.addProperty("resource_location", resLoc.toString())

                val possibleStates = block.stateDefinition.possibleStates.toMutableList()
                    .apply { sortBy { it.id } }
                val statePropertyCombinations = if (block.stateDefinition.properties.isNotEmpty()) {
                    block.stateDefinition.properties.map { it.possibleValues.size }.reduce(Int::times)
                } else 1
                check(possibleStates.size == statePropertyCombinations) {
                    "Number of possible states (${possibleStates.size}) doesn't match the number of state property combinations ($statePropertyCombinations)"
                }
                val idBase = possibleStates.first().id.also { blockData.addProperty("id_base", it) }
                blockData.addProperty("total_states", possibleStates.size)
                blockData.addProperty("default_state", block.defaultBlockState().id - idBase)

                val blockstatePropertiesData = JsonArray().also { blockData.add("blockstate_properties", it) }
                for (property in block.stateDefinition.properties) {
                    val blockstatePropertyData = JsonObject().also { blockstatePropertiesData.add(it) }

                    blockstatePropertyData.addProperty("name", property.name)

                    when (property) {
                        is BooleanProperty -> {
                            blockstatePropertyData.addProperty("type", "boolean")
                            check(property.possibleValues == listOf(true, false))
                        }

                        is IntegerProperty -> {
                            blockstatePropertyData.addProperty("type", "integer")
                            check(property.possibleValues.zipWithNext().all { (a, b) -> a < b }) {
                                "IntegerProperty possible values are not sorted??"
                            }
                            blockstatePropertyData.addProperty("min", property.possibleValues.first())
                            blockstatePropertyData.addProperty("max", property.possibleValues.last())
                        }

                        is EnumProperty -> {
                            blockstatePropertyData.addProperty("type", "enum")
                            blockstatePropertyData.add(
                                "values",
                                JsonArray().also { arr ->
                                    property.possibleValues.forEach { arr.add(it.serializedName) }
                                }
                            )
                        }
                    }

                    val idGroupSize =
                        possibleStates.takeWhile { it.getValue(property) == property.possibleValues.first() }.size

                    blockstatePropertyData.addProperty("id_group_size", idGroupSize)

                    // check validity
                    possibleStates.forEach { check(it.getValue(property) == property.possibleValues[(it.id - idBase) / idGroupSize % property.possibleValues.size]) }
                }

                DataProvider.saveStable(
                    cachedOutput,
                    blockData,
                    packOutput.outputFolder
                        .resolve("block_def")
                        .resolve(resLoc.namespace)
                        .resolve("${resLoc.path}.json")
                )
            }.toArray { n -> arrayOfNulls<CompletableFuture<*>>(n) }
        )
    }

    override fun getName() = "TileGlobeMC: BlockDef"
}