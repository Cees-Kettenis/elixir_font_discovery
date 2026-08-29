defmodule ElixirFontDiscovery.AdapterTest do
  use ExUnit.Case

  alias ElixirFontDiscovery.Adapter

  defmodule FakeNative do
    def resolve(_family, _weight, _style) do
      case Process.get(:font_discovery_native_result) do
        :font -> {:ok, "Installed Sans", 650.0, :oblique, "font"}
        :missing -> {:error, :not_found}
        :unsupported -> {:error, :unsupported_font}
        :unavailable -> {:error, :unavailable}
        :unexpected -> {:ok, nil}
        :raise -> :erlang.nif_error(:nif_not_loaded)
        :exit -> exit(:native_failure)
      end
    end
  end

  test "normalizes a valid native result" do
    Process.put(:font_discovery_native_result, :font)

    assert Adapter.resolve("sans-serif", 650, :oblique, FakeNative) ==
             {:ok, %{family: "Installed Sans", weight: 650.0, style: :oblique, data: "font"}}
  end

  test "preserves a missing-family result" do
    Process.put(:font_discovery_native_result, :missing)

    assert Adapter.resolve("missing", 400, :normal, FakeNative) == {:error, :not_found}
  end

  test "preserves an unsupported-font result" do
    Process.put(:font_discovery_native_result, :unsupported)

    assert Adapter.resolve("Variable Sans", 700, :normal, FakeNative) ==
             {:error, :unsupported_font}
  end

  test "turns unusable or unavailable native results into unavailable errors" do
    Process.put(:font_discovery_native_result, :unavailable)
    assert Adapter.resolve("sans-serif", 400, :normal, FakeNative) == {:error, :unavailable}

    Process.put(:font_discovery_native_result, :unexpected)
    assert Adapter.resolve("sans-serif", 400, :normal, FakeNative) == {:error, :unavailable}

    Process.put(:font_discovery_native_result, :raise)
    assert Adapter.resolve("sans-serif", 400, :normal, FakeNative) == {:error, :unavailable}

    Process.put(:font_discovery_native_result, :exit)
    assert Adapter.resolve("sans-serif", 400, :normal, FakeNative) == {:error, :unavailable}
  end
end
