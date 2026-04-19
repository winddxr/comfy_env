pub mod env;
pub mod init;
pub mod install;
pub mod node;
pub mod ops;
pub mod pin;
pub mod resolve;
pub mod runtime;
pub mod status;
pub mod tx;
pub mod undo;
pub mod update;

use crate::cli::{
    Cli, Commands, EnvSubcommand, InstallSubcommand, NodeSubcommand, OpSubcommand, PinSubcommand,
    TxSubcommand, UpdateSubcommand,
};
use crate::domain::{AppResult, ProjectRoot};

pub fn dispatch(cli: Cli) -> AppResult<()> {
    let root = ProjectRoot::discover()?;

    match cli.command {
        Commands::Init(args) => init::cmd_init(args, &root),
        Commands::Install(args) => match args.command {
            Some(InstallSubcommand::Torch(torch_args)) => {
                install::cmd_install_torch(torch_args, &root)
            }
            None => install::cmd_install(args, &root),
        },
        Commands::Pin(args) => match args.command {
            PinSubcommand::Add(add_args) => pin::cmd_pin_add(add_args, &root),
            PinSubcommand::List => pin::cmd_pin_list(&root),
            PinSubcommand::Remove(remove_args) => pin::cmd_pin_remove(remove_args, &root),
        },
        Commands::Node(args) => match args.command {
            NodeSubcommand::Add(add_args) => node::cmd_node_add(add_args, &root),
            NodeSubcommand::Remove(remove_args) => node::cmd_node_remove(remove_args, &root),
        },
        Commands::Tx(args) => match args.command {
            TxSubcommand::Run(run_args) => tx::cmd_tx_run(run_args, &root),
            TxSubcommand::Inspect(inspect_args) => tx::cmd_tx_inspect(inspect_args, &root),
            TxSubcommand::Abort(abort_args) => tx::cmd_tx_abort(abort_args, &root),
            TxSubcommand::Promote(promote_args) => tx::cmd_tx_promote(promote_args, &root),
        },
        Commands::Update(args) => match args.command {
            UpdateSubcommand::Run(run_args) => update::cmd_update_run(run_args, &root),
            UpdateSubcommand::Inspect(inspect_args) => {
                update::cmd_update_inspect(inspect_args, &root)
            }
            UpdateSubcommand::Abort(abort_args) => update::cmd_update_abort(abort_args, &root),
            UpdateSubcommand::Promote(promote_args) => {
                update::cmd_update_promote(promote_args, &root)
            }
            UpdateSubcommand::Resolve(resolve_args) => {
                update::cmd_update_resolve(resolve_args, &root)
            }
        },
        Commands::Resolve(args) => resolve::cmd_resolve(args, &root),
        Commands::Env(args) => match args.command {
            EnvSubcommand::Export(export_args) => env::cmd_env_export(export_args, &root),
            EnvSubcommand::Import(import_args) => env::cmd_env_import(import_args, &root),
        },
        Commands::Op(args) => match args.command {
            OpSubcommand::List => ops::cmd_op_list(&root),
            OpSubcommand::Inspect(inspect_args) => ops::cmd_op_inspect(inspect_args, &root),
        },
        Commands::Undo(args) => undo::cmd_undo(args, &root),
        Commands::Run(args) => runtime::cmd_run(args, &root),
        Commands::Stop => runtime::cmd_stop(&root),
        Commands::Status => status::cmd_status(&root),
    }
}
