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
  version = cargoToml.package.version;

  src = builtins.path {
    path = ../.;
  };

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
       "svg-0.18.0" = "sha256-4U0ZtrevD5KIdtxO8/z80hwOmzOAKNtC19yIr/FrNzY=";
       "pathfinder_simd-0.5.4" = "sha256-1IIMAow7bw0kmbaJUp8GLaKo7Hx/QzYSQ2dE93wqlDc=";
    };
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
  };
}
