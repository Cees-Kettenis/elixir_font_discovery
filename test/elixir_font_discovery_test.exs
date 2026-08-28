defmodule ElixirFontDiscoveryTest do
  use ExUnit.Case

  doctest ElixirFontDiscovery

  test "resolves an installed generic sans-serif face" do
    assert {:ok, font} = ElixirFontDiscovery.resolve("sans-serif")
    assert font.family != ""
    assert font.weight > 0
    assert font.style in [:normal, :italic, :oblique]
    assert <<0, 1, 0, 0, _rest::binary>> = font.data
  end

  test "resolves a requested weight and style" do
    assert {:ok, font} = ElixirFontDiscovery.resolve("sans-serif", 700, :italic)
    assert font.family != ""
    assert font.weight > 0
    assert font.style in [:normal, :italic, :oblique]
    assert byte_size(font.data) > 0
  end

  test "reports a missing named family" do
    assert ElixirFontDiscovery.resolve("A Font That Is Not Installed 123") ==
             {:error, :not_found}
  end

  test "validates requests before entering the native adapter" do
    assert ElixirFontDiscovery.resolve(123) == {:error, :invalid_family}
    assert ElixirFontDiscovery.resolve("  ") == {:error, :invalid_family}

    assert ElixirFontDiscovery.resolve("sans-serif", 0, :normal) ==
             {:error, :invalid_weight}

    assert ElixirFontDiscovery.resolve("sans-serif", 1001, :normal) ==
             {:error, :invalid_weight}

    assert ElixirFontDiscovery.resolve("sans-serif", :heavy, :normal) ==
             {:error, :invalid_weight}

    assert ElixirFontDiscovery.resolve("sans-serif", 400, :slanted) ==
             {:error, :invalid_style}
  end
end
