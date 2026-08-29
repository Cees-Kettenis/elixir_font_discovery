defmodule ElixirFontDiscovery.Native do
  @moduledoc false

  mix_config = Mix.Project.config()
  version = mix_config[:version]
  github_url = mix_config[:package][:links]["GitHub"]

  use RustlerPrecompiled,
    otp_app: :elixir_font_discovery,
    crate: "elixir_font_discovery",
    base_url: "#{github_url}/releases/download/v#{version}",
    force_build: System.get_env("ELIXIR_FONT_DISCOVERY_BUILD") in ["1", "true"],
    version: version,
    targets: ~w(
      aarch64-apple-darwin
      aarch64-unknown-linux-gnu
      x86_64-apple-darwin
      x86_64-pc-windows-gnu
      x86_64-pc-windows-msvc
      x86_64-unknown-linux-gnu
    )s

  @doc false
  @spec resolve(String.t(), number(), ElixirFontDiscovery.style()) ::
          {:ok, String.t(), float(), ElixirFontDiscovery.style(), binary()}
          | {:error, :not_found | :unsupported_font | :unavailable}
  def resolve(_family, _weight, _style), do: :erlang.nif_error(:nif_not_loaded)
end
