defmodule ElixirFontDiscovery do
  @moduledoc """
  Resolves installed font families through the host operating system.

  Linux uses Fontconfig, macOS uses CoreText, and Windows uses DirectWrite.
  Published releases download a verified precompiled native library, so callers
  do not need Rust or a native compiler.
  """

  alias ElixirFontDiscovery.Adapter

  @typedoc "A style requested from or reported by the operating system."
  @type style :: :normal | :italic | :oblique

  @typedoc "An installed font face and its standalone OpenType or TrueType bytes."
  @type font :: %{
          family: String.t(),
          weight: float(),
          style: style(),
          data: binary()
        }

  @typedoc "Why an installed font could not be returned."
  @type reason ::
          :invalid_family
          | :invalid_weight
          | :invalid_style
          | :not_found
          | :unavailable

  @doc """
  Resolves an installed regular face for `family` at CSS weight 400.

  Generic families such as `sans-serif`, `serif`, and `monospace` are resolved
  by the operating system.
  """
  @spec resolve(String.t()) :: {:ok, font()} | {:error, reason()}
  def resolve(family) do
    resolve(family, 400, :normal)
  end

  @doc """
  Resolves the closest installed face for a family, CSS weight, and style.

  `weight` must be between 1 and 1000. The accepted styles are `:normal`,
  `:italic`, and `:oblique`. The returned family, weight, and style describe the
  face selected by the operating system, which can differ from the request.
  """
  @spec resolve(String.t(), number(), style()) :: {:ok, font()} | {:error, reason()}
  def resolve(family, weight, style) do
    with :ok <- validate_family(family),
         :ok <- validate_weight(weight),
         :ok <- validate_style(style) do
      Adapter.resolve(String.trim(family), weight, style)
    end
  end

  defp validate_family(family) do
    case family do
      value when is_binary(value) ->
        if String.trim(value) == "", do: {:error, :invalid_family}, else: :ok

      _other ->
        {:error, :invalid_family}
    end
  end

  defp validate_weight(weight) do
    case weight do
      value when is_number(value) and value >= 1 and value <= 1000 -> :ok
      _other -> {:error, :invalid_weight}
    end
  end

  defp validate_style(style) do
    if style in [:normal, :italic, :oblique], do: :ok, else: {:error, :invalid_style}
  end
end
