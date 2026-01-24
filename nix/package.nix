{
  lib,
  stdenv,
  makeRustPlatform,
  rust-bin,
  installShellFiles,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  toolchain = rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
  rustPlatform = makeRustPlatform {
    cargo = toolchain;
    rustc = toolchain;
  };
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = builtins.path { path = ../.; };

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = import ./cargo-hashes.nix;
  };

  nativeBuildInputs = [ installShellFiles ];

  postInstall = ''
    installShellCompletion --cmd termframe \
    --bash <($out/bin/termframe --shell-completions bash) \
    --fish <($out/bin/termframe --shell-completions fish) \
    --zsh <($out/bin/termframe --shell-completions zsh)
    $out/bin/termframe --man-page >termframe.1
    installManPage termframe.1
  '';

  doCheck = false;

  meta = {
    description = cargoToml.package.description;
    homepage = cargoToml.package.repository;
    license = lib.licenses.mit;
    mainProgram = cargoToml.package.name;
    maintainers = [
      {
        name = "Pavel Ivanov";
        github = "pamburus";
        email = "mr.pavel.ivanov@gmail.com";
      }
    ];
  };
}
