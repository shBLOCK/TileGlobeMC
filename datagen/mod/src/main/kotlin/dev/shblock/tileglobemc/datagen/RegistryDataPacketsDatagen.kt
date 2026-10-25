package dev.shblock.tileglobemc.datagen

import io.netty.buffer.ByteBufUtil
import io.netty.buffer.Unpooled
import net.minecraft.core.RegistrySynchronization
import net.minecraft.nbt.NbtOps
import net.minecraft.network.FriendlyByteBuf
import net.minecraft.network.PacketListener
import net.minecraft.network.protocol.Packet
import net.minecraft.network.protocol.common.ClientboundUpdateTagsPacket
import net.minecraft.network.protocol.configuration.ClientConfigurationPacketListener
import net.minecraft.network.protocol.configuration.ClientboundRegistryDataPacket
import net.minecraft.network.protocol.configuration.ConfigurationProtocols
import net.minecraft.server.MinecraftServer
import net.minecraft.server.RegistryLayer
import net.minecraft.server.packs.repository.KnownPack
import net.minecraft.tags.TagNetworkSerialization
import java.io.File
import java.io.FileOutputStream

fun genRegistryDataPackets(server: MinecraftServer) {
    val registries = server.registries()
    val data = FriendlyByteBuf(Unpooled.buffer())

    fun writeConfigurationPacket(packet: Packet<in ClientConfigurationPacketListener>) {
        val packetBuf = FriendlyByteBuf(Unpooled.buffer())
        ConfigurationProtocols.CLIENTBOUND.codec().encode(packetBuf, packet)
        data.writeVarInt(packetBuf.readableBytes())
        data.writeBytes(packetBuf)
    }

    RegistrySynchronization.packRegistries(
        registries.compositeAccess().createSerializationContext(NbtOps.INSTANCE),
        registries.getAccessFrom(RegistryLayer.WORLD),
        setOf(KnownPack.vanilla("core"))
    ) { registryKey, entries ->
        writeConfigurationPacket(ClientboundRegistryDataPacket(registryKey, entries))
    }

    writeConfigurationPacket(ClientboundUpdateTagsPacket(TagNetworkSerialization.serializeTagsToNetwork(registries)))

    FileOutputStream(File("../generated/additional/registry_data_packets.dat")).use { file ->
        file.write(ByteBufUtil.getBytes(data))
    }
}
