package dev.shblock.tileglobemc

import dev.shblock.tileglobemc.datagen.BlockDefDatagen
import net.neoforged.bus.api.SubscribeEvent
import net.neoforged.fml.common.EventBusSubscriber
import net.neoforged.fml.common.Mod
import net.neoforged.neoforge.data.event.GatherDataEvent
import org.apache.logging.log4j.Level
import org.apache.logging.log4j.LogManager
import org.apache.logging.log4j.Logger

@Mod(TileGlobeMC.ID)
@EventBusSubscriber(modid = TileGlobeMC.ID)
object TileGlobeMC {
    const val ID = "tileglobemc"

    val LOGGER: Logger = LogManager.getLogger(ID)

    init {
        LOGGER.log(Level.INFO, "Hello world!")
    }

    @SubscribeEvent
    fun onGatherDataServer(event: GatherDataEvent.Server) {
        event.createProvider(::BlockDefDatagen)
    }
}
