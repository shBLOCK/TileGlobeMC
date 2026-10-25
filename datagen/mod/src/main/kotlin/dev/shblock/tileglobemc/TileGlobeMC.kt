package dev.shblock.tileglobemc

import dev.shblock.tileglobemc.datagen.genRegistryDataPackets
import net.fabricmc.api.DedicatedServerModInitializer
import net.fabricmc.api.ModInitializer
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents
import net.minecraft.resources.Identifier
import org.slf4j.LoggerFactory

object TileGlobeMC : ModInitializer, DedicatedServerModInitializer {
	const val MOD_ID: String = "tileglobemc"

	private val LOGGER = LoggerFactory.getLogger(MOD_ID)

	override fun onInitialize() {
	}

    override fun onInitializeServer() {
        ServerLifecycleEvents.SERVER_STARTED.register { server ->
            genRegistryDataPackets(server)
            
            Runtime.getRuntime().halt(0);
        }
    }

    fun id(path: String): Identifier
		= Identifier.fromNamespaceAndPath(MOD_ID, path)
}
