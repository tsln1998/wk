use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Host {
    Table,
    Id,
    MachineId,
    MachineIp,
    MachineCountry,
    MachineGeo,
    OsFamily,
    OsName,
    OsVersion,
    OsArch,
    OsBuild,
    OsVirtualization,
    HashedCPU,
    HashedGPU,
    HashedMemory,
    HashedDisk,
    HashedNetwork,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Host::Table)
                    .if_not_exists()
                    .col(pk_uuid(Host::Id))
                    .col(string(Host::MachineId))
                    .col(string(Host::MachineIp).string_len(45))
                    .col(string(Host::MachineCountry).string_len(3))
                    .col(string(Host::MachineGeo).string_len(255))
                    .col(string(Host::OsFamily).string_len(8))
                    .col(string(Host::OsName).string_len(64))
                    .col(string(Host::OsVersion).string_len(64))
                    .col(string(Host::OsArch).string_len(8))
                    .col(string(Host::OsBuild).string_len(64))
                    .col(boolean(Host::OsVirtualization))
                    .col(integer(Host::HashedCPU))
                    .col(integer(Host::HashedGPU))
                    .col(integer(Host::HashedMemory))
                    .col(integer(Host::HashedDisk))
                    .col(integer(Host::HashedNetwork))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Host::Table).to_owned())
            .await
    }
}
