defmodule ElixirFontDiscovery.MixProject do
  use Mix.Project

  @version "0.1.1"
  @source_url "https://github.com/Cees-Kettenis/elixir_font_discovery"

  def project do
    [
      app: :elixir_font_discovery,
      version: @version,
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      description: "Cross-platform discovery of installed fonts for Elixir.",
      package: package(),
      deps: deps(),
      docs: docs(),
      source_url: @source_url,
      homepage_url: @source_url,
      dialyzer: [plt_add_apps: [:mix]],
      test_coverage: [ignore_modules: [ElixirFontDiscovery.Native]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      {:rustler_precompiled, ">= 0.8.1 and < 1.0.0"},
      {:rustler, "~> 0.36.2", optional: true},
      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false},
      {:ex_doc, "~> 0.38", only: :dev, runtime: false}
    ]
  end

  defp package do
    [
      files: ~w(
        lib
        native/elixir_font_discovery/.cargo
        native/elixir_font_discovery/src
        native/elixir_font_discovery/Cargo.lock
        native/elixir_font_discovery/Cargo.toml
        checksum-*.exs
        CHANGELOG.md
        LICENSE
        README.md
        mix.exs
      )s,
      licenses: ["MIT"],
      links: %{"GitHub" => @source_url},
      maintainers: ["Cees Kettenis"]
    ]
  end

  defp docs do
    [
      main: "readme",
      extras: ["README.md", "CHANGELOG.md", "LICENSE"],
      source_ref: "v#{@version}"
    ]
  end
end
