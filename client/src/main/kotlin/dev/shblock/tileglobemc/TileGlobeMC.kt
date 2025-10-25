package dev.shblock.tileglobemc

import net.neoforged.api.distmarker.Dist
import net.neoforged.fml.common.EventBusSubscriber
import net.neoforged.fml.common.Mod
import org.apache.logging.log4j.Level
import org.apache.logging.log4j.LogManager
import org.apache.logging.log4j.Logger

@Mod(TileGlobeMC.ID, dist = [Dist.CLIENT])
@EventBusSubscriber(Dist.CLIENT)
object TileGlobeMC {
    const val ID = "tileglobemc"

    val LOGGER: Logger = LogManager.getLogger(ID)

    init {
        LOGGER.log(Level.INFO, "Hello world!")
    }
}
