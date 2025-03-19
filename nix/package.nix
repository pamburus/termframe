{
  lib,
  stdenv,
  rustPlatform,
  installShellFiles,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.workspace.package.version;

  src = builtins.path {
    path = ../.;
  };

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {};
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
    homepage = cargoToml.workspace.package.repository;
    license = lib.licenses.mit;
    mainProgram = cargoToml.package.name;
  };
}
