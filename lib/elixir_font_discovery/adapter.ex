defmodule ElixirFontDiscovery.Adapter do
  @moduledoc false

  alias ElixirFontDiscovery.Native

  @doc false
  @spec resolve(String.t(), number(), ElixirFontDiscovery.style()) ::
          {:ok, ElixirFontDiscovery.font()} | {:error, :not_found | :unavailable}
  def resolve(family, weight, style) do
    resolve(family, weight, style, Native)
  end

  @doc false
  @spec resolve(String.t(), number(), ElixirFontDiscovery.style(), module()) ::
          {:ok, ElixirFontDiscovery.font()} | {:error, :not_found | :unavailable}
  def resolve(family, weight, style, native) do
    try do
      case native.resolve(family, weight, style) do
        {:ok, resolved_family, resolved_weight, resolved_style, data}
        when is_binary(resolved_family) and resolved_family != "" and
               is_number(resolved_weight) and
               resolved_style in [:normal, :italic, :oblique] and is_binary(data) and
               byte_size(data) > 0 ->
          {:ok,
           %{
             family: resolved_family,
             weight: resolved_weight / 1,
             style: resolved_style,
             data: data
           }}

        {:error, :not_found} ->
          {:error, :not_found}

        _unexpected ->
          {:error, :unavailable}
      end
    rescue
      ErlangError -> {:error, :unavailable}
    catch
      :exit, _reason -> {:error, :unavailable}
    end
  end
end
