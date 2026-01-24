{
  lib,
  stdenv,
  fetchurl,
  installShellFiles,
}:

let
  # Get the current version from Cargo.toml (for metadata only)
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);

  # Import release hashes from separate file (automatically updated by GitHub Actions)
  releaseHashes = import ./binary-hashes.nix;

  # Get the highest version available in binary-hashes.nix
  availableVersions = builtins.attrNames releaseHashes;

  # Fail if no versions are available in binary-hashes.nix
  version =
    if availableVersions == [ ] then
      throw "No versions available in binary-hashes.nix - package configuration is inconsistent"
    else
      builtins.foldl' (
        acc: v: if builtins.compareVersions v acc > 0 then v else acc
      ) (builtins.head availableVersions) availableVersions;

  # Map Nix system to asset name
  getAssetName =
    system:
    {
      "x86_64-linux" = "termframe-linux-x86_64-musl.tar.gz";
      "aarch64-linux" = "termframe-linux-arm64-musl.tar.gz";
      "x86_64-darwin" = "termframe-macos-x86_64.tar.gz";
      "aarch64-darwin" = "termframe-macos-arm64.tar.gz";
    }
    .${system} or (throw "Unsupported system: ${system}");

  # Build the download URL
  assetName = getAssetName stdenv.hostPlatform.system;
  downloadUrl = "https://github.com/pamburus/termframe/releases/download/v${version}/${assetName}";

  # Get hash for the current version and asset
  assetHash =
    releaseHashes.${version}.${assetName}
      or (throw "No hash available for version ${version} and asset ${assetName} - package configuration is inconsistent");

  # Fetch the binary
  src = fetchurl {
    url = downloadUrl;
    sha256 = assetHash;
  };

in
stdenv.mkDerivation {
  pname = "termframe-bin";
  inherit version;

  inherit src;

  nativeBuildInputs = [ installShellFiles ];

  unpackPhase = ''
    runHook preUnpack

    case "$src" in
      *.tar.gz)
        tar -xzf "$src"
        ;;
      *.zip)
        unzip "$src"
        ;;
      *)
        echo "Unsupported archive format"
        exit 1
        ;;
    esac

    runHook postUnpack
  '';

  installPhase = ''
    runHook preInstall

    # Install the binary
    install -D -m755 termframe $out/bin/termframe

    # Generate and install shell completions
    installShellCompletion --cmd termframe \
      --bash <($out/bin/termframe --shell-completions bash) \
      --fish <($out/bin/termframe --shell-completions fish) \
      --zsh <($out/bin/termframe --shell-completions zsh)

    # Generate and install man page
    $out/bin/termframe --man-page >termframe.1
    installManPage termframe.1

    runHook postInstall
  '';

  meta = {
    description = "${cargoToml.package.description} (binary distribution)";
    homepage = cargoToml.workspace.package.repository;
    license = lib.licenses.mit;
    changelog = "${cargoToml.workspace.package.repository}/releases";
    mainProgram = cargoToml.package.name;
    maintainers = [
      {
        name = "Pavel Ivanov";
        github = "pamburus";
        email = "mr.pavel.ivanov@gmail.com";
      }
    ];
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
  };
}
