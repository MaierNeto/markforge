use std::collections::BTreeMap;

use lopdf::content::{Content, Operation};
use lopdf::{Document, Encoding, Object};

/// Quanto de recuo (em milésimos de em) entre dois pedaços de um mesmo `TJ`
/// conta como espaço entre palavras. O `TJ` intercala texto e deslocamentos:
/// os pequenos são ajuste fino entre letras (kerning), os grandes é que
/// separam palavras. Ignorar essa diferença cola palavras ("umtexto") ou
/// espalha espaço no meio delas.
const RECUO_DE_ESPACO: f32 = 150.0;

/// Acima desta fração de texto ilegível o documento não é aproveitável, e
/// gravar o que saiu seria entregar lixo com cara de conteúdo.
const LIMITE_DE_ILEGIVEL: f32 = 0.5;

/// A partir de quantas vezes o corpo um tamanho de fonte entra na faixa de
/// títulos. É um compromisso: alto demais perde título de terceiro nível
/// (documento real usa proporção curta — 13pt sobre corpo de 11pt é título),
/// baixo demais promove destaque e legenda a título.
const PISO_DE_TITULO: f32 = 1.15;

#[derive(Debug, Clone)]
pub struct TextElement {
    pub text: String,
    pub x: f32,
    pub y: f32,
    /// Número da página de origem. **Sem isto o documento embaralha:** o topo
    /// da página 2 tem Y maior que o rodapé da página 1, e uma ordenação só
    /// por Y joga a página seguinte para dentro da anterior.
    pub page: u32,
    pub font_name: String,
    pub font_size: f32,
    pub bold: bool,
    /// Verdadeiro quando a fonte deste trecho não pôde ser decodificada e o
    /// texto saiu como caractere de substituição.
    pub ilegivel: bool,
}

/// Mapa `ToUnicode` de uma fonte: código de caractere → texto.
///
/// **Por que não usamos o do `lopdf`.** Ele resolve `Identity-H` lendo o
/// `ToUnicode`, mas o parser de CMap dele rejeita CMaps perfeitamente
/// regulares — inclusive o que o Typst gera, isto é, o do PDF que o próprio
/// Markforge exporta. Quando isso acontece o encoding se perde e o texto CID
/// cru vira sequência de controle: lixo com aparência de conteúdo. Ler o
/// `ToUnicode` aqui cobre o caso mais comum em documento de verdade (fonte
/// embutida em subconjunto, vinda de Word, LibreOffice ou Typst).
#[derive(Debug, Clone, Default)]
pub struct MapaUnicode {
    mapa: BTreeMap<u32, String>,
    /// Quantos bytes formam um código (1 para fonte simples, 2 para CID).
    bytes_por_codigo: usize,
}

impl MapaUnicode {
    /// Lê um CMap de `ToUnicode`: aceita `beginbfchar` (pares código→destino) e
    /// `beginbfrange`, nas duas formas — faixa contígua e lista entre `[ ]`.
    pub fn ler(conteudo: &[u8]) -> Option<Self> {
        let texto = String::from_utf8_lossy(conteudo);
        let mut mapa = BTreeMap::new();
        let mut bytes_por_codigo = 2;

        if let Some(faixa) = trecho_entre(&texto, "begincodespacerange", "endcodespacerange") {
            if let Some(primeiro) = hexadecimais(faixa).first() {
                // `<00> <FF>` = 1 byte por código; `<0000> <FFFF>` = 2 bytes.
                bytes_por_codigo = (primeiro.len() / 2).clamp(1, 2);
            }
        }

        for bloco in trechos_entre(&texto, "beginbfchar", "endbfchar") {
            let itens = hexadecimais(bloco);
            for par in itens.chunks(2) {
                if let [codigo, destino] = par {
                    if let (Some(c), Some(d)) = (codigo_de(codigo), texto_de(destino)) {
                        mapa.insert(c, d);
                    }
                }
            }
        }

        for bloco in trechos_entre(&texto, "beginbfrange", "endbfrange") {
            for (inicio, fim, destinos) in faixas(bloco) {
                for (passo, codigo) in (inicio..=fim).enumerate() {
                    let destino = match destinos.len() {
                        // Lista explícita: um destino por código da faixa.
                        n if n > 1 => destinos.get(passo).cloned(),
                        // Faixa contígua: o destino anda junto com o código.
                        1 => texto_deslocado(&destinos[0], passo as u32),
                        _ => None,
                    };
                    if let Some(destino) = destino {
                        mapa.insert(codigo, destino);
                    }
                }
            }
        }

        (!mapa.is_empty()).then_some(Self { mapa, bytes_por_codigo })
    }

    /// Converte a sequência de códigos em texto. Código fora do mapa vira o
    /// caractere de substituição — visível, nunca silencioso.
    pub fn decodificar(&self, bytes: &[u8]) -> String {
        bytes
            .chunks(self.bytes_por_codigo)
            .map(|pedaco| {
                let codigo = pedaco.iter().fold(0u32, |acc, b| (acc << 8) | *b as u32);
                self.mapa.get(&codigo).cloned().unwrap_or_else(|| "\u{FFFD}".to_string())
            })
            .collect()
    }
}

fn trecho_entre<'a>(texto: &'a str, abre: &str, fecha: &str) -> Option<&'a str> {
    let inicio = texto.find(abre)? + abre.len();
    let fim = texto[inicio..].find(fecha)? + inicio;
    Some(&texto[inicio..fim])
}

fn trechos_entre<'a>(texto: &'a str, abre: &str, fecha: &str) -> Vec<&'a str> {
    let mut blocos = Vec::new();
    let mut resto = texto;
    while let Some(i) = resto.find(abre) {
        let depois = &resto[i + abre.len()..];
        match depois.find(fecha) {
            Some(f) => {
                blocos.push(&depois[..f]);
                resto = &depois[f + fecha.len()..];
            }
            None => break,
        }
    }
    blocos
}

/// Todos os `<...>` de um trecho, sem os sinais.
fn hexadecimais(trecho: &str) -> Vec<String> {
    let mut itens = Vec::new();
    let mut resto = trecho;
    while let Some(i) = resto.find('<') {
        let depois = &resto[i + 1..];
        match depois.find('>') {
            Some(f) => {
                itens.push(depois[..f].split_whitespace().collect::<String>());
                resto = &depois[f + 1..];
            }
            None => break,
        }
    }
    itens
}

/// Faixas de um `bfrange`: `<lo> <hi> <destino>` ou `<lo> <hi> [<d> <d> …]`.
fn faixas(trecho: &str) -> Vec<(u32, u32, Vec<String>)> {
    let mut saida = Vec::new();
    for linha in trecho.lines() {
        let linha = linha.trim();
        if linha.is_empty() {
            continue;
        }
        let itens = hexadecimais(linha);
        if itens.len() < 3 {
            continue;
        }
        let (Some(inicio), Some(fim)) = (codigo_de(&itens[0]), codigo_de(&itens[1])) else {
            continue;
        };
        let destinos: Vec<String> = itens[2..].iter().filter_map(|h| texto_de(h)).collect();
        if !destinos.is_empty() && inicio <= fim {
            saida.push((inicio, fim, destinos));
        }
    }
    saida
}

fn codigo_de(hex: &str) -> Option<u32> {
    u32::from_str_radix(hex, 16).ok()
}

/// Destino de um mapeamento: UTF-16BE, podendo ter mais de um caractere
/// (ligadura "ﬁ" → "fi", por exemplo).
fn texto_de(hex: &str) -> Option<String> {
    if hex.len() % 4 != 0 || hex.is_empty() {
        return None;
    }
    let unidades: Vec<u16> = (0..hex.len() / 4)
        .filter_map(|i| u16::from_str_radix(&hex[i * 4..i * 4 + 4], 16).ok())
        .collect();
    String::from_utf16(&unidades).ok()
}

/// Destino de faixa contígua: o último caractere anda `passo` posições.
fn texto_deslocado(destino: &str, passo: u32) -> Option<String> {
    let mut chars: Vec<char> = destino.chars().collect();
    let ultimo = chars.pop()?;
    let deslocado = char::from_u32(ultimo as u32 + passo)?;
    chars.push(deslocado);
    Some(chars.into_iter().collect())
}

/// Como decodificar o texto de uma fonte. `ToUnicode` primeiro (é o que o
/// gerador do PDF declarou), `lopdf` depois (cobre as fontes de encoding
/// padrão), e a ausência dos dois é registrada — não vira palpite.
pub struct Fonte<'a> {
    pub nome: String,
    mapa: Option<MapaUnicode>,
    encoding: Option<Encoding<'a>>,
}

impl<'a> Fonte<'a> {
    /// `None` no retorno significa fonte que não sabemos ler.
    fn decodificar(&self, bytes: &[u8]) -> Option<String> {
        if let Some(mapa) = &self.mapa {
            return Some(mapa.decodificar(bytes));
        }
        self.encoding.as_ref().and_then(|e| e.bytes_to_string(bytes).ok())
    }
}

fn is_bold(font_name: &str) -> bool {
    let n = font_name.to_ascii_lowercase();
    n.contains("bold") || n.contains("black") || n.contains("heavy") || n.ends_with("-b") || n.ends_with(",b")
}

fn numero(objeto: &Object) -> Option<f32> {
    match objeto {
        Object::Real(v) => Some(*v),
        Object::Integer(v) => Some(*v as f32),
        _ => None,
    }
}

const IDENTIDADE: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// Compõe duas matrizes do PDF (`primeira` aplicada antes de `segunda`).
/// A matriz `[a b c d e f]` do PDF é a matriz 3×3 com terceira coluna fixa.
fn compor(primeira: [f32; 6], segunda: [f32; 6]) -> [f32; 6] {
    let [a1, b1, c1, d1, e1, f1] = primeira;
    let [a2, b2, c2, d2, e2, f2] = segunda;
    [
        a1 * a2 + b1 * c2,
        a1 * b2 + b1 * d2,
        c1 * a2 + d1 * c2,
        c1 * b2 + d1 * d2,
        e1 * a2 + f1 * c2 + e2,
        e1 * b2 + f1 * d2 + f2,
    ]
}

/// Estado de texto de uma página, na parte que interessa para reconstruir
/// linha e posição. `BT` zera; `Tm` define; `Td`/`TD`/`T*`/`'`/`"` andam a
/// partir do **início da linha**, não da posição corrente — acumular sobre a
/// posição corrente escorrega o texto para a direita a cada linha.
struct EstadoDeTexto {
    linha: [f32; 6],
    corrente: [f32; 6],
    entrelinha: f32,
}

impl EstadoDeTexto {
    fn novo() -> Self {
        Self { linha: IDENTIDADE, corrente: IDENTIDADE, entrelinha: 0.0 }
    }

    fn definir_matriz(&mut self, m: [f32; 6]) {
        self.linha = m;
        self.corrente = m;
    }

    fn nova_linha(&mut self, tx: f32, ty: f32) {
        self.linha[4] += tx;
        self.linha[5] += ty;
        self.corrente = self.linha;
    }

    fn proxima_linha(&mut self) {
        let entrelinha = self.entrelinha;
        self.nova_linha(0.0, -entrelinha);
    }
}

/// Onde o texto realmente cai na página: a matriz de texto **composta com a
/// matriz gráfica** (`cm`).
///
/// **Por que isto importa.** Vários geradores — reportlab e o desenho por
/// estado gráfico em geral — deixam a matriz de texto em `0 0` e põem a
/// posição verdadeira da linha no `cm`, dentro de um `q … Q`. Lendo só a
/// matriz de texto, todo o documento aparece empilhado quase no mesmo ponto,
/// e nada que dependa de posição (linha, parágrafo, coluna) funciona.
fn posicao(texto: [f32; 6], grafica: [f32; 6]) -> (f32, f32, f32) {
    let m = compor(texto, grafica);
    // A escala do texto é a raiz do determinante: vale também quando a matriz
    // gira o texto, em que ler só o termo vertical daria zero.
    let escala = (m[0] * m[3] - m[1] * m[2]).abs().sqrt();
    (m[4], m[5], if escala > 0.0 { escala } else { 1.0 })
}

/// Percorre os operadores de uma página e devolve os trechos de texto com
/// posição e fonte. Separado da leitura do arquivo para poder ser testado com
/// operadores montados à mão.
pub fn elementos_da_pagina(
    operacoes: &[Operation],
    fontes: &BTreeMap<Vec<u8>, Fonte<'_>>,
    pagina: u32,
) -> Vec<TextElement> {
    let mut elementos = Vec::new();
    let mut estado = EstadoDeTexto::novo();
    let mut fonte_atual: Option<&Fonte> = None;
    let mut corpo = 0.0f32;
    let mut grafica = IDENTIDADE;
    let mut pilha_grafica: Vec<[f32; 6]> = Vec::new();

    let mut mostrar = |estado: &EstadoDeTexto,
                       grafica: [f32; 6],
                       fonte: Option<&Fonte>,
                       corpo: f32,
                       partes: &[Parte]| {
        let Some(fonte) = fonte else { return };
        let mut texto = String::new();
        let mut ilegivel = false;
        for parte in partes {
            match parte {
                Parte::Bytes(bytes) => match fonte.decodificar(bytes) {
                    Some(t) => texto.push_str(&t),
                    None => {
                        ilegivel = true;
                        texto.push('\u{FFFD}');
                    }
                },
                Parte::Recuo(v) => {
                    if *v <= -RECUO_DE_ESPACO && !texto.ends_with(' ') {
                        texto.push(' ');
                    }
                }
            }
        }
        if texto.trim().is_empty() {
            return;
        }
        let (x, y, escala) = posicao(estado.corrente, grafica);
        elementos.push(TextElement {
            text: texto,
            x,
            y,
            page: pagina,
            font_name: fonte.nome.clone(),
            font_size: corpo * escala,
            bold: is_bold(&fonte.nome),
            ilegivel,
        });
    };

    for op in operacoes {
        match op.operator.as_str() {
            "q" => pilha_grafica.push(grafica),
            "Q" => grafica = pilha_grafica.pop().unwrap_or(IDENTIDADE),
            "cm" => {
                let m: Vec<f32> = op.operands.iter().filter_map(numero).collect();
                if m.len() >= 6 {
                    grafica = compor([m[0], m[1], m[2], m[3], m[4], m[5]], grafica);
                }
            }
            "BT" => estado = EstadoDeTexto::novo(),
            "Tf" => {
                if let Some(Object::Name(nome)) = op.operands.first() {
                    fonte_atual = fontes.get(nome.as_slice());
                }
                if let Some(v) = op.operands.get(1).and_then(numero) {
                    corpo = v;
                }
            }
            "TL" => {
                if let Some(v) = op.operands.first().and_then(numero) {
                    estado.entrelinha = v;
                }
            }
            "Tm" => {
                let m: Vec<f32> = op.operands.iter().filter_map(numero).collect();
                if m.len() >= 6 {
                    estado.definir_matriz([m[0], m[1], m[2], m[3], m[4], m[5]]);
                }
            }
            "Td" | "TD" => {
                let tx = op.operands.first().and_then(numero).unwrap_or(0.0);
                let ty = op.operands.get(1).and_then(numero).unwrap_or(0.0);
                // `TD` também fixa a entrelinha para os `T*` seguintes.
                if op.operator == "TD" {
                    estado.entrelinha = -ty;
                }
                estado.nova_linha(tx, ty);
            }
            "T*" => estado.proxima_linha(),
            "Tj" => {
                if let Some(Object::String(bytes, _)) = op.operands.first() {
                    mostrar(&estado, grafica, fonte_atual, corpo, &[Parte::Bytes(bytes.clone())]);
                }
            }
            // `'` e `"` mostram texto **na linha seguinte**. Sem tratá-los, todo
            // o texto de geradores que os usam simplesmente some.
            "'" | "\"" => {
                estado.proxima_linha();
                if let Some(Object::String(bytes, _)) = op.operands.last() {
                    mostrar(&estado, grafica, fonte_atual, corpo, &[Parte::Bytes(bytes.clone())]);
                }
            }
            "TJ" => {
                if let Some(Object::Array(itens)) = op.operands.first() {
                    let partes: Vec<Parte> = itens
                        .iter()
                        .filter_map(|item| match item {
                            Object::String(bytes, _) => Some(Parte::Bytes(bytes.clone())),
                            outro => numero(outro).map(Parte::Recuo),
                        })
                        .collect();
                    mostrar(&estado, grafica, fonte_atual, corpo, &partes);
                }
            }
            _ => {}
        }
    }

    elementos
}

enum Parte {
    Bytes(Vec<u8>),
    Recuo(f32),
}

/// Extrai texto com posição e fonte de um PDF.
///
/// A ordem final é **página, depois topo-para-baixo, depois esquerda-para-
/// direita**. As coordenadas do PDF crescem para cima, por isso o Y é
/// comparado invertido.
pub fn extract_pdf_text_with_positions(path: &str) -> Result<Vec<TextElement>, String> {
    let doc = Document::load(path).map_err(|e| format!("Falha ao carregar PDF: {e}"))?;
    let mut elementos = Vec::new();

    for (numero_da_pagina, page_id) in doc.get_pages() {
        let conteudo = doc.get_page_content(page_id).map_err(|e| e.to_string())?;
        let conteudo = Content::decode(&conteudo).map_err(|e| e.to_string())?;

        let mut fontes: BTreeMap<Vec<u8>, Fonte> = BTreeMap::new();
        for (nome, dicionario) in doc.get_page_fonts(page_id).map_err(|e| e.to_string())? {
            let mapa = dicionario
                .get_deref(b"ToUnicode", &doc)
                .ok()
                .and_then(|o| o.as_stream().ok())
                .and_then(|s| s.decompressed_content().ok())
                .and_then(|bytes| MapaUnicode::ler(&bytes));
            let nome_da_fonte = dicionario
                .get(b"BaseFont")
                .and_then(|o| o.as_name())
                .map(|n| String::from_utf8_lossy(n).to_string())
                .unwrap_or_else(|_| String::from_utf8_lossy(&nome).to_string());
            fontes.insert(
                nome,
                Fonte {
                    nome: nome_da_fonte,
                    mapa,
                    encoding: dicionario.get_font_encoding(&doc).ok(),
                },
            );
        }

        elementos.extend(elementos_da_pagina(&conteudo.operations, &fontes, numero_da_pagina));
    }

    elementos.sort_by(|a, b| {
        a.page
            .cmp(&b.page)
            .then_with(|| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    Ok(elementos)
}

/// Recusa o documento quando a maior parte do texto saiu ilegível. Gravar um
/// `.md` de caracteres de substituição seria pior do que dizer que não deu:
/// o usuário acharia que importou.
fn conferir_legibilidade(elementos: &[TextElement]) -> Result<(), String> {
    if elementos.is_empty() {
        return Ok(());
    }
    let ilegiveis = elementos.iter().filter(|e| e.ilegivel).count() as f32;
    if ilegiveis / elementos.len() as f32 > LIMITE_DE_ILEGIVEL {
        return Err(
            "Não foi possível ler o texto deste PDF — as fontes do arquivo não trazem o mapa de \
             caracteres. Se ele for um documento digitalizado, será preciso reconhecer o texto antes."
                .to_string(),
        );
    }
    Ok(())
}

/// Coleta os tamanhos de fonte que representam headings: distintos, a partir do
/// piso de título sobre o corpo dominante, ordenados desc. Os três maiores
/// viram H1/H2/H3.
fn collect_heading_sizes(sizes: &[f32], body_size: f32) -> Vec<f32> {
    let mut hs: Vec<f32> = sizes
        .iter()
        .copied()
        .filter(|&s| s >= PISO_DE_TITULO * body_size)
        .collect();
    hs.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    hs.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    hs.truncate(3);
    hs
}

/// Devolve o nível de heading (1..3) para um tamanho de fonte, ou `None` se for
/// corpo. `heading_sizes_desc` vem de `collect_heading_sizes`.
fn heading_level_for_size(size: f32, heading_sizes_desc: &[f32]) -> Option<usize> {
    heading_sizes_desc
        .iter()
        .position(|&s| (s - size).abs() < 0.5)
        .map(|rank| rank + 1)
}

/// Tamanho de fonte dominante (modo por arredondamento de 0.5pt), determinístico:
/// em empate de frequência escolhe o menor, para que o corpo normal não seja
/// sequestrado por um título pontual.
fn dominant_size(sizes: &[f32]) -> f32 {
    use std::collections::HashMap;
    let mut buckets: HashMap<i32, (usize, f32)> = HashMap::new();
    let mut fallback = sizes.first().copied().unwrap_or(10.0);
    for &s in sizes {
        if s <= 0.0 {
            continue;
        }
        fallback = s;
        let key = (s / 0.5).round() as i32;
        let entry = buckets.entry(key).or_insert((0, s));
        entry.0 += 1;
    }
    let max_count = buckets.values().map(|(c, _)| *c).max().unwrap_or(0);
    buckets
        .into_iter()
        .filter(|(_, (c, _))| *c == max_count)
        .map(|(_, (_, size))| size)
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(fallback)
}

/// Uma linha visual da página: o que estava na mesma altura, já juntado.
///
/// **Por que a linha é a unidade.** Raciocinar em elemento solto não permite
/// distinguir "próxima linha do mesmo parágrafo" de "parágrafo novo", nem
/// "rótulo com texto ao lado" de "título sozinho na linha" — e é daí que vem
/// tanto o picadinho de parágrafo quanto o excesso de título.
struct Linha {
    page: u32,
    y: f32,
    texto: String,
    tamanho: f32,
    negrito: bool,
    /// Quantos trechos formaram a linha. `1` significa que a linha inteira é
    /// aquele texto — evidência de que ele é título, e não rótulo de campo.
    pedacos: usize,
}

/// Diferença de Y que ainda conta como a mesma linha (sobrescrito, mudança de
/// fonte no meio da frase).
const TOLERANCIA_DE_LINHA: f32 = 2.0;

/// Quanto a folga precisa passar da entrelinha para ser troca de parágrafo.
const FATOR_DE_PARAGRAFO: f32 = 1.5;

/// Em quantas páginas o mesmo texto, na mesma altura, denuncia cabeçalho ou
/// rodapé em vez de conteúdo.
const PAGINAS_PARA_SER_ELEMENTO_DE_PAGINA: usize = 3;

const MARCADORES: [char; 7] = ['•', '·', '▪', '◦', '‣', '–', '—'];

/// Fração da largura de linha do documento até onde um título pode ir.
/// **Título não ocupa a linha inteira** — quem chega à margem é texto corrido.
/// É a evidência que separa "seção" de "frase em fonte um pouco maior", e sem
/// ela um documento cujo corpo não é o tamanho mais frequente (nota de rodapé
/// e citação abundantes, por exemplo) promove parágrafos inteiros a título.
const FRACAO_DE_LINHA_PARA_TITULO: f32 = 0.7;

/// Abaixo disto não há linhas suficientes para medir a largura do documento, e
/// a regra acima ficaria à mercê de uma amostra pequena.
const LINHAS_PARA_MEDIR_LARGURA: usize = 20;

/// Comprimento máximo de um título, medido no próprio documento. `None` quando
/// o documento é curto demais para a medida ser confiável.
fn limite_de_titulo(linhas: &[Linha]) -> Option<usize> {
    if linhas.len() < LINHAS_PARA_MEDIR_LARGURA {
        return None;
    }
    let mut larguras: Vec<usize> = linhas.iter().map(|l| l.texto.chars().count()).collect();
    larguras.sort_unstable();
    let cheia = larguras[larguras.len() * 9 / 10];
    Some((cheia as f32 * FRACAO_DE_LINHA_PARA_TITULO) as usize)
}

/// Junta um trecho ao que já existe na linha, decidindo o espaço. Espaço antes
/// de vírgula ou ponto é erro de digitação, não separação de palavra.
fn juntar_na_linha(acumulado: &mut String, novo: &str) {
    if acumulado.is_empty() {
        acumulado.push_str(novo);
        return;
    }
    let comeca_com_pontuacao = novo.starts_with([',', '.', ';', ':', '!', '?', ')', ']', '}', '%', '»', '…']);
    let termina_aberto = acumulado.ends_with(['(', '[', '{', '«', '/', '-', '¿', '¡']);
    let ja_tem_espaco = acumulado.ends_with(' ') || novo.starts_with(' ');
    if !comeca_com_pontuacao && !termina_aberto && !ja_tem_espaco {
        acumulado.push(' ');
    }
    acumulado.push_str(novo);
}

/// Agrupa os elementos (já ordenados por página, topo e esquerda) em linhas.
fn agrupar_em_linhas(elements: &[TextElement]) -> Vec<Linha> {
    let mut linhas: Vec<Linha> = Vec::new();
    for el in elements {
        let texto = el.text.trim();
        if texto.is_empty() {
            continue;
        }
        match linhas.last_mut() {
            Some(linha)
                if linha.page == el.page && (linha.y - el.y).abs() <= TOLERANCIA_DE_LINHA =>
            {
                juntar_na_linha(&mut linha.texto, texto);
                linha.tamanho = linha.tamanho.max(el.font_size);
                linha.negrito = linha.negrito && (el.bold || is_bold(&el.font_name));
                linha.pedacos += 1;
            }
            _ => linhas.push(Linha {
                page: el.page,
                y: el.y,
                texto: texto.to_string(),
                tamanho: el.font_size,
                negrito: el.bold || is_bold(&el.font_name),
                pedacos: 1,
            }),
        }
    }
    linhas
}

/// A entrelinha do documento: a **menor** folga entre linhas seguidas que ainda
/// seja compatível com o corpo do texto. Tomar a menor (e não a mais comum)
/// acerta também o documento em que quase todo parágrafo tem uma linha só —
/// ali a folga mais comum é a de parágrafo, e usá-la juntaria o texto inteiro
/// num bloco. O piso pelo corpo descarta a folga curta de índice e expoente.
fn entrelinha(linhas: &[Linha], corpo: f32) -> f32 {
    linhas
        .windows(2)
        .filter(|par| par[0].page == par[1].page)
        .map(|par| par[0].y - par[1].y)
        .filter(|d| *d >= 0.9 * corpo)
        .fold(f32::INFINITY, f32::min)
        .min(2.0 * corpo)
}

/// Descarta cabeçalho e rodapé: mesmo texto, mesma altura, em várias páginas.
/// Repetição em **alturas diferentes** é conteúdo e fica.
fn sem_elementos_de_pagina(linhas: Vec<Linha>) -> Vec<Linha> {
    let mut paginas_por_texto: std::collections::HashMap<(&str, i32), Vec<u32>> =
        std::collections::HashMap::new();
    for linha in &linhas {
        paginas_por_texto
            .entry((linha.texto.as_str(), (linha.y / 2.0).round() as i32))
            .or_default()
            .push(linha.page);
    }
    let repetidos: std::collections::HashSet<(&str, i32)> = paginas_por_texto
        .into_iter()
        .filter(|(_, paginas)| {
            let distintas: std::collections::HashSet<u32> = paginas.iter().copied().collect();
            distintas.len() >= PAGINAS_PARA_SER_ELEMENTO_DE_PAGINA
        })
        .map(|(chave, _)| chave)
        .collect();
    if repetidos.is_empty() {
        return linhas;
    }
    let descartar: Vec<(String, i32)> =
        repetidos.into_iter().map(|(t, y)| (t.to_string(), y)).collect();
    linhas
        .into_iter()
        .filter(|linha| {
            !descartar
                .iter()
                .any(|(t, y)| *t == linha.texto && *y == (linha.y / 2.0).round() as i32)
        })
        .collect()
}

/// Marcador de lista no início da linha, se houver: devolve o resto do texto.
fn item_de_lista(texto: &str) -> Option<&str> {
    let resto = texto
        .strip_prefix(MARCADORES)
        .or_else(|| texto.strip_prefix("- "))
        .or_else(|| texto.strip_prefix("* "))?;
    let resto = resto.trim_start();
    (!resto.is_empty()).then_some(resto)
}

#[derive(PartialEq)]
enum Bloco {
    Nenhum,
    Paragrafo,
    Item,
    Titulo,
}

/// Converte elementos extraídos para Markdown estruturado (GFM).
pub fn elements_to_markdown(elements: &[TextElement]) -> String {
    if elements.is_empty() {
        return String::new();
    }

    let sizes: Vec<f32> = elements.iter().map(|e| e.font_size).collect();
    let corpo = dominant_size(&sizes);
    let tamanhos_de_titulo = collect_heading_sizes(&sizes, corpo);

    let linhas = sem_elementos_de_pagina(agrupar_em_linhas(elements));
    if linhas.is_empty() {
        return String::new();
    }
    let entrelinha = entrelinha(&linhas, corpo);
    let limite_de_titulo = limite_de_titulo(&linhas);

    /// Nível de título da linha, ou `None` para corpo.
    ///
    /// Fora o tamanho de fonte, **título ocupa a linha inteira**. Sem essa
    /// exigência, rótulo de campo ("Assunto:" com o teor ao lado) e negrito no
    /// meio da frase viram título — e cada um deles é um marcador falso no PDF
    /// exportado.
    fn nivel_de_titulo(
        linha: &Linha,
        tamanhos: &[f32],
        corpo: f32,
        limite: Option<usize>,
    ) -> Option<usize> {
        if limite.is_some_and(|max| linha.texto.chars().count() > max) {
            return None;
        }
        if let Some(nivel) = heading_level_for_size(linha.tamanho, tamanhos) {
            return Some(nivel);
        }
        if linha.pedacos > 1 {
            return None;
        }
        if linha.negrito && linha.tamanho >= 1.1 * corpo {
            return Some(2);
        }
        let dois_pontos = linha.texto.ends_with(':')
            && linha.texto.len() <= 60
            && !linha.texto.contains(' ')
            && !linha.texto.chars().all(|c| c == ':');
        dois_pontos.then_some(2)
    }

    let mut md = String::new();
    let mut anterior = Bloco::Nenhum;
    let mut ultima: Option<&Linha> = None;

    for linha in &linhas {
        if let Some(nivel) = nivel_de_titulo(linha, &tamanhos_de_titulo, corpo, limite_de_titulo) {
            if anterior != Bloco::Nenhum {
                md.push_str("\n\n");
            }
            md.push_str(&format!("{} {}", "#".repeat(nivel), linha.texto.trim_end_matches(':')));
            anterior = Bloco::Titulo;
            ultima = Some(linha);
            continue;
        }

        if let Some(resto) = item_de_lista(&linha.texto) {
            md.push_str(match anterior {
                Bloco::Nenhum => "",
                // Itens seguidos formam uma lista só.
                Bloco::Item => "\n",
                _ => "\n\n",
            });
            md.push_str("- ");
            md.push_str(resto);
            anterior = Bloco::Item;
            ultima = Some(linha);
            continue;
        }

        // Continua o parágrafo quando a linha vem logo abaixo, na mesma página
        // e na entrelinha do documento.
        let continua = anterior == Bloco::Paragrafo
            && ultima.is_some_and(|u| {
                let folga = u.y - linha.y;
                u.page == linha.page && folga > 0.0 && folga <= entrelinha * FATOR_DE_PARAGRAFO
            });

        if continua {
            // Palavra partida pela diagramação: o hífen não é do texto.
            if md.ends_with('-') && linha.texto.starts_with(|c: char| c.is_lowercase()) {
                md.pop();
                md.push_str(&linha.texto);
            } else {
                juntar_na_linha(&mut md, &linha.texto);
            }
        } else {
            if anterior != Bloco::Nenhum {
                md.push_str("\n\n");
            }
            md.push_str(&linha.texto);
        }
        anterior = Bloco::Paragrafo;
        ultima = Some(linha);
    }

    md.trim().to_string()
}

/// Importa um PDF convertendo para Markdown (GFM estruturado).
/// Usado pelo pipeline `import_document`, que grava o arquivo e confere colisão.
pub fn import_pdf_to_markdown(path: &str) -> Result<String, String> {
    let elements = extract_pdf_text_with_positions(path)?;
    conferir_legibilidade(&elements)?;
    Ok(elements_to_markdown(&elements))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> String {
        format!("../docs/_interno/pdf-fixtures/{name}")
    }

    fn elemento(text: &str, y: f32, size: f32, bold: bool) -> TextElement {
        TextElement {
            text: text.into(),
            x: 0.0,
            y,
            page: 1,
            font_name: if bold { "Helvetica-Bold".into() } else { "Helvetica".into() },
            font_size: size,
            bold,
            ilegivel: false,
        }
    }

    // --- Mapa ToUnicode -----------------------------------------------------

    const CMAP_BFCHAR: &str = "\
        1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
        2 beginbfchar\n<0001> <004D>\n<0002> <0061>\nendbfchar\n";

    #[test]
    fn mapa_unicode_le_bfchar() {
        let mapa = MapaUnicode::ler(CMAP_BFCHAR.as_bytes()).expect("deveria ler o CMap");
        assert_eq!(mapa.decodificar(&[0x00, 0x01, 0x00, 0x02]), "Ma");
    }

    #[test]
    fn mapa_unicode_le_faixa_contigua() {
        let cmap = "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                    1 beginbfrange\n<0010> <0012> <0041>\nendbfrange\n";
        let mapa = MapaUnicode::ler(cmap.as_bytes()).unwrap();
        assert_eq!(mapa.decodificar(&[0x00, 0x10, 0x00, 0x11, 0x00, 0x12]), "ABC");
    }

    #[test]
    fn mapa_unicode_le_faixa_com_lista_de_destinos() {
        let cmap = "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                    1 beginbfrange\n<0020> <0022> [<0058> <0059> <005A>]\nendbfrange\n";
        let mapa = MapaUnicode::ler(cmap.as_bytes()).unwrap();
        assert_eq!(mapa.decodificar(&[0x00, 0x20, 0x00, 0x21, 0x00, 0x22]), "XYZ");
    }

    /// Ligadura: um código que vale por dois caracteres.
    #[test]
    fn mapa_unicode_le_destino_de_varios_caracteres() {
        let cmap = "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                    1 beginbfchar\n<0005> <00660069>\nendbfchar\n";
        let mapa = MapaUnicode::ler(cmap.as_bytes()).unwrap();
        assert_eq!(mapa.decodificar(&[0x00, 0x05]), "fi");
    }

    #[test]
    fn mapa_unicode_respeita_codespace_de_um_byte() {
        let cmap = "1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
                    1 beginbfchar\n<41> <0061>\nendbfchar\n";
        let mapa = MapaUnicode::ler(cmap.as_bytes()).unwrap();
        assert_eq!(mapa.decodificar(&[0x41]), "a");
    }

    /// Código fora do mapa não pode virar caractere plausível — vira sinal.
    #[test]
    fn mapa_unicode_marca_codigo_desconhecido() {
        let mapa = MapaUnicode::ler(CMAP_BFCHAR.as_bytes()).unwrap();
        assert_eq!(mapa.decodificar(&[0x00, 0x99]), "\u{FFFD}");
    }

    // --- Percurso dos operadores -------------------------------------------

    fn fontes_de_teste() -> BTreeMap<Vec<u8>, Fonte<'static>> {
        let mut fontes = BTreeMap::new();
        fontes.insert(
            b"F1".to_vec(),
            Fonte {
                nome: "LibertinusSerif".into(),
                mapa: MapaUnicode::ler(
                    "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                     3 beginbfchar\n<0001> <0061>\n<0002> <0062>\n<0003> <0063>\nendbfchar\n"
                        .as_bytes(),
                ),
                encoding: None,
            },
        );
        fontes
    }

    fn op(operador: &str, operandos: Vec<Object>) -> Operation {
        Operation::new(operador, operandos)
    }

    fn mostrar(codigo: u8) -> Object {
        Object::String(vec![0x00, codigo], lopdf::StringFormat::Literal)
    }

    /// `T*` avança uma entrelinha — e o texto seguinte tem de cair na linha de
    /// baixo, não em cima da anterior.
    #[test]
    fn percurso_trata_proxima_linha() {
        let ops = vec![
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(10)]),
            op("TL", vec![Object::Integer(12)]),
            op("Td", vec![Object::Integer(50), Object::Integer(700)]),
            op("Tj", vec![mostrar(1)]),
            op("T*", vec![]),
            op("Tj", vec![mostrar(2)]),
        ];
        let els = elementos_da_pagina(&ops, &fontes_de_teste(), 1);
        assert_eq!(els.len(), 2);
        assert_eq!(els[0].text, "a");
        assert_eq!(els[1].text, "b");
        assert_eq!(els[1].y, 688.0, "T* deveria descer uma entrelinha");
    }

    /// O operador `'` mostra texto na linha seguinte. Antes ele era ignorado e
    /// o texto sumia sem aviso.
    #[test]
    fn percurso_trata_aspas_como_mostrar_na_proxima_linha() {
        let ops = vec![
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(10)]),
            op("TL", vec![Object::Integer(12)]),
            op("Td", vec![Object::Integer(50), Object::Integer(700)]),
            op("'", vec![mostrar(3)]),
        ];
        let els = elementos_da_pagina(&ops, &fontes_de_teste(), 1);
        assert_eq!(els.len(), 1, "texto de ' não pode sumir");
        assert_eq!(els[0].text, "c");
        assert_eq!(els[0].y, 688.0);
    }

    /// Duas linhas com `Td` andam a partir do início da linha anterior, não da
    /// posição corrente — senão a segunda linha escorrega para a direita.
    #[test]
    fn percurso_nao_acumula_deslocamento_na_posicao_corrente() {
        let ops = vec![
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(10)]),
            op("Tm", vec![
                Object::Integer(1), Object::Integer(0), Object::Integer(0),
                Object::Integer(1), Object::Integer(72), Object::Integer(700),
            ]),
            op("Tj", vec![mostrar(1)]),
            op("Td", vec![Object::Integer(0), Object::Integer(-12)]),
            op("Tj", vec![mostrar(2)]),
        ];
        let els = elementos_da_pagina(&ops, &fontes_de_teste(), 1);
        assert_eq!(els[1].x, 72.0, "a segunda linha começa na mesma margem");
        assert_eq!(els[1].y, 688.0);
    }

    /// Recuo grande dentro do `TJ` é espaço entre palavras; recuo pequeno é
    /// ajuste entre letras e não pode virar espaço.
    #[test]
    fn percurso_transforma_recuo_grande_em_espaco() {
        let ops = vec![
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(10)]),
            op("TJ", vec![Object::Array(vec![
                mostrar(1),
                Object::Integer(-300),
                mostrar(2),
                Object::Integer(-20),
                mostrar(3),
            ])]),
        ];
        let els = elementos_da_pagina(&ops, &fontes_de_teste(), 1);
        assert_eq!(els.len(), 1, "um TJ é um trecho só");
        assert_eq!(els[0].text, "a bc");
    }

    /// `BT` zera a matriz de texto: o bloco seguinte não herda a posição do
    /// anterior.
    #[test]
    fn percurso_zera_a_matriz_em_cada_bloco() {
        let ops = vec![
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(10)]),
            op("Td", vec![Object::Integer(50), Object::Integer(700)]),
            op("Tj", vec![mostrar(1)]),
            op("ET", vec![]),
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(10)]),
            op("Tj", vec![mostrar(2)]),
        ];
        let els = elementos_da_pagina(&ops, &fontes_de_teste(), 1);
        assert_eq!(els[1].x, 0.0);
        assert_eq!(els[1].y, 0.0);
    }

    /// Corpo 1pt escalado 12× é corpo 12pt — é assim que vários geradores
    /// escrevem, e a régua de título depende de enxergar isso.
    #[test]
    fn percurso_multiplica_o_corpo_pela_escala_da_matriz() {
        let ops = vec![
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(1)]),
            op("Tm", vec![
                Object::Integer(12), Object::Integer(0), Object::Integer(0),
                Object::Integer(12), Object::Integer(72), Object::Integer(700),
            ]),
            op("Tj", vec![mostrar(1)]),
        ];
        let els = elementos_da_pagina(&ops, &fontes_de_teste(), 1);
        assert_eq!(els[0].font_size, 12.0);
    }

    /// **Padrão real de gerador que posiciona pelo estado gráfico:** a matriz
    /// de texto fica em `0 4` e a posição verdadeira da linha está no `cm`.
    /// Lendo só a matriz de texto, a página inteira se empilha num ponto só.
    #[test]
    fn percurso_soma_a_matriz_grafica_a_posicao() {
        let ops = vec![
            op("q", vec![]),
            op("cm", vec![
                Object::Integer(1), Object::Integer(0), Object::Integer(0),
                Object::Integer(1), Object::Integer(78), Object::Real(741.9),
            ]),
            op("BT", vec![]),
            op("Tm", vec![
                Object::Integer(1), Object::Integer(0), Object::Integer(0),
                Object::Integer(1), Object::Integer(0), Object::Integer(4),
            ]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(18)]),
            op("Tj", vec![mostrar(1)]),
        ];
        let els = elementos_da_pagina(&ops, &fontes_de_teste(), 1);
        assert_eq!(els[0].x, 78.0);
        assert_eq!(els[0].y, 745.9, "a posição é a composição das duas matrizes");
        assert_eq!(els[0].font_size, 18.0, "matriz sem escala não altera o corpo");
    }

    /// `Q` devolve a matriz gráfica de antes do `q` — sem isso o deslocamento
    /// de um bloco vaza para todos os seguintes.
    #[test]
    fn percurso_restaura_a_matriz_grafica_ao_fechar_o_bloco() {
        let ops = vec![
            op("q", vec![]),
            op("cm", vec![
                Object::Integer(1), Object::Integer(0), Object::Integer(0),
                Object::Integer(1), Object::Integer(78), Object::Integer(700),
            ]),
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(10)]),
            op("Tj", vec![mostrar(1)]),
            op("Q", vec![]),
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(10)]),
            op("Tj", vec![mostrar(2)]),
        ];
        let els = elementos_da_pagina(&ops, &fontes_de_teste(), 1);
        let fora = els.iter().find(|e| e.text == "b").expect("texto fora do bloco");
        assert_eq!((fora.x, fora.y), (0.0, 0.0));
    }

    /// Fonte sem `ToUnicode` e sem encoding conhecido não pode virar texto
    /// plausível: fica marcada.
    #[test]
    fn percurso_marca_texto_de_fonte_ilegivel() {
        let mut fontes = BTreeMap::new();
        fontes.insert(
            b"F1".to_vec(),
            Fonte { nome: "Desconhecida".into(), mapa: None, encoding: None },
        );
        let ops = vec![
            op("BT", vec![]),
            op("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(10)]),
            op("Tj", vec![mostrar(1)]),
        ];
        let els = elementos_da_pagina(&ops, &fontes, 1);
        assert!(els[0].ilegivel, "trecho de fonte desconhecida deveria estar marcado");
    }

    // --- Ordem e legibilidade ----------------------------------------------

    /// A página é a primeira chave de ordenação. Sem ela o topo da página 2
    /// (Y alto) se enfia antes do rodapé da página 1 (Y baixo).
    #[test]
    fn ordem_respeita_a_pagina_antes_do_y() {
        let mut els = vec![
            TextElement { page: 2, y: 780.0, ..elemento("topo da pagina 2", 780.0, 10.0, false) },
            TextElement { page: 1, y: 60.0, ..elemento("rodape da pagina 1", 60.0, 10.0, false) },
        ];
        els.sort_by(|a, b| {
            a.page
                .cmp(&b.page)
                .then_with(|| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal))
        });
        assert_eq!(els[0].text, "rodape da pagina 1");
    }

    #[test]
    fn documento_majoritariamente_ilegivel_e_recusado() {
        let ilegivel = |t: &str| TextElement { ilegivel: true, ..elemento(t, 100.0, 10.0, false) };
        let els = vec![ilegivel("\u{FFFD}"), ilegivel("\u{FFFD}"), elemento("ok", 90.0, 10.0, false)];
        assert!(conferir_legibilidade(&els).is_err());
    }

    #[test]
    fn documento_legivel_passa() {
        let els = vec![elemento("texto de verdade", 100.0, 10.0, false)];
        assert!(conferir_legibilidade(&els).is_ok());
    }

    // --- Títulos e Markdown -------------------------------------------------

    #[test]
    fn test_heading_level_ranking() {
        let sizes = vec![18.0, 14.0, 12.0, 10.0, 10.0, 10.0];
        let hs = collect_heading_sizes(&sizes, 10.0);
        assert_eq!(hs, vec![18.0, 14.0, 12.0]);
        assert_eq!(heading_level_for_size(18.0, &hs), Some(1));
        assert_eq!(heading_level_for_size(14.0, &hs), Some(2));
        assert_eq!(heading_level_for_size(12.0, &hs), Some(3));
        assert_eq!(heading_level_for_size(10.0, &hs), None);
    }

    #[test]
    fn test_collect_heading_sizes_ignores_large_body() {
        let sizes = vec![12.0, 10.0, 10.0, 10.0, 10.0];
        let hs = collect_heading_sizes(&sizes, 10.0);
        assert_eq!(hs, vec![12.0]);
        assert_eq!(heading_level_for_size(12.0, &hs), Some(1));
        assert_eq!(heading_level_for_size(10.0, &hs), None);
    }

    #[test]
    fn test_elements_to_markdown_headings() {
        let elements = vec![
            elemento("Relatório de Vendas", 100.0, 18.0, true),
            elemento("Este documento apresenta os resultados.", 90.0, 10.0, false),
            elemento("Primeiro Trimestre", 80.0, 14.0, true),
        ];
        let md = elements_to_markdown(&elements);
        assert!(md.contains("# Relatório de Vendas"), "H1 deveria virar '# '");
        assert!(md.contains("Este documento apresenta os resultados."));
        assert!(md.contains("## Primeiro Trimestre"), "H2 deveria virar '## '");
    }

    #[test]
    fn test_elements_to_markdown_section_title_colon() {
        let elements = vec![
            elemento("Resumo:", 100.0, 11.0, false),
            elemento("Conteúdo do resumo.", 90.0, 10.0, false),
        ];
        let md = elements_to_markdown(&elements);
        assert!(md.contains("## Resumo"), "Título por prefixo ':' deveria virar '## '");
        assert!(md.contains("Conteúdo do resumo."));
    }

    /// Fim de uma página e começo da outra são parágrafos diferentes, mesmo
    /// quando os Y são parecidos.
    #[test]
    fn markdown_quebra_paragrafo_na_virada_de_pagina() {
        let elements = vec![
            TextElement { page: 1, ..elemento("Fim da primeira pagina.", 100.0, 10.0, false) },
            TextElement { page: 2, ..elemento("Comeco da segunda.", 100.0, 10.0, false) },
        ];
        let md = elements_to_markdown(&elements);
        assert!(
            md.contains("Fim da primeira pagina.\n\nComeco da segunda."),
            "virada de página deveria quebrar parágrafo, saiu:\n{md}"
        );
    }

    // --- Linha × parágrafo, espaçamento e ruído de página --------------------
    //
    // Padrões reproduzidos de documento real (o conteúdo aqui é sintético: o
    // que se registra é o mecanismo, nunca a amostra).

    /// Elemento com posição livre — para montar linha e coluna à mão.
    fn em(text: &str, x: f32, y: f32, size: f32, bold: bool, page: u32) -> TextElement {
        TextElement {
            text: text.into(),
            x,
            y,
            page,
            font_name: if bold { "Fonte-Bold".into() } else { "Fonte".into() },
            font_size: size,
            bold,
            ilegivel: false,
        }
    }

    /// **O defeito principal.** Linhas seguidas na entrelinha normal são o
    /// mesmo parágrafo. Quebrar cada uma vira picadinho: frase começando em
    /// minúscula, bloco sem pontuação final.
    #[test]
    fn linhas_seguidas_formam_um_paragrafo_so() {
        let els = vec![
            em("O documento trata de um assunto", 50.0, 700.0, 11.0, false, 1),
            em("que continua nesta linha e", 50.0, 687.0, 11.0, false, 1),
            em("termina nesta aqui.", 50.0, 674.0, 11.0, false, 1),
        ];
        let md = elements_to_markdown(&els);
        assert_eq!(
            md, "O documento trata de um assunto que continua nesta linha e termina nesta aqui.",
            "linhas na entrelinha normal são um parágrafo só"
        );
    }

    /// Folga maior que a entrelinha é troca de parágrafo — esse é o sinal que
    /// separa um bloco do outro.
    #[test]
    fn folga_maior_que_a_entrelinha_quebra_paragrafo() {
        let els = vec![
            em("Primeiro paragrafo do texto.", 50.0, 700.0, 11.0, false, 1),
            em("Ainda o primeiro.", 50.0, 687.0, 11.0, false, 1),
            em("Ja e o segundo paragrafo.", 50.0, 650.0, 11.0, false, 1),
        ];
        let md = elements_to_markdown(&els);
        assert_eq!(md, "Primeiro paragrafo do texto. Ainda o primeiro.\n\nJa e o segundo paragrafo.");
    }

    /// Palavra partida no fim da linha se remonta — o hífen é da diagramação,
    /// não do texto.
    #[test]
    fn hifen_no_fim_da_linha_remonta_a_palavra() {
        let els = vec![
            em("um exem-", 50.0, 700.0, 11.0, false, 1),
            em("plo de palavra partida.", 50.0, 687.0, 11.0, false, 1),
        ];
        assert_eq!(elements_to_markdown(&els), "um exemplo de palavra partida.");
    }

    /// Trechos da mesma linha se juntam com espaço — menos antes de
    /// pontuação, onde o espaço é erro de digitação.
    #[test]
    fn nao_insere_espaco_antes_de_pontuacao() {
        let els = vec![
            em("uma frase", 50.0, 700.0, 11.0, false, 1),
            em("destacada", 120.0, 700.0, 11.0, true, 1),
            em(".", 180.0, 700.0, 11.0, false, 1),
        ];
        assert_eq!(elements_to_markdown(&els), "uma frase destacada.");
    }

    /// **Rótulo não é seção.** "Assunto:" com texto ao lado é campo de
    /// formulário; virar `##` enche o PDF exportado de marcador falso.
    #[test]
    fn rotulo_com_texto_ao_lado_nao_vira_titulo() {
        let els = vec![
            em("Assunto:", 50.0, 700.0, 11.0, false, 1),
            em("o teor da mensagem", 110.0, 700.0, 11.0, false, 1),
        ];
        let md = elements_to_markdown(&els);
        assert!(!md.contains('#'), "rótulo com texto ao lado não é título, saiu:\n{md}");
    }

    /// Mas título de seção com dois-pontos, sozinho na linha, continua título.
    #[test]
    fn titulo_com_dois_pontos_sozinho_na_linha_continua_titulo() {
        let els = vec![
            em("Resumo:", 50.0, 700.0, 11.0, false, 1),
            em("O conteudo do resumo vem aqui.", 50.0, 687.0, 11.0, false, 1),
        ];
        assert!(elements_to_markdown(&els).starts_with("## Resumo"));
    }

    /// Negrito no meio da linha é ênfase, não título.
    #[test]
    fn negrito_com_texto_ao_lado_nao_vira_titulo() {
        let els = vec![
            em("veja o", 50.0, 700.0, 11.0, false, 1),
            em("prazo limite", 90.0, 700.0, 12.0, true, 1),
            em("para o envio.", 160.0, 700.0, 11.0, false, 1),
        ];
        let md = elements_to_markdown(&els);
        assert!(!md.contains('#'), "negrito inline não é título, saiu:\n{md}");
    }

    /// O marcador já vem no texto; falta virar lista de verdade.
    #[test]
    fn marcador_de_lista_vira_item() {
        let els = vec![
            em("• primeiro item", 50.0, 700.0, 11.0, false, 1),
            em("• segundo item", 50.0, 687.0, 11.0, false, 1),
        ];
        assert_eq!(elements_to_markdown(&els), "- primeiro item\n- segundo item");
    }

    /// **Título não chega à margem.** Num documento cujo corpo não é o tamanho
    /// mais frequente, frases inteiras num tamanho um pouco maior seriam
    /// promovidas a título — e cada uma vira um marcador falso no PDF
    /// exportado. A largura da própria linha desmente isso.
    #[test]
    fn linha_que_ocupa_a_largura_toda_nao_e_titulo() {
        let cheia = "palavra ".repeat(13); // ~104 caracteres, largura de coluna
        let mut els: Vec<TextElement> = (0..24)
            .map(|i| em(&cheia, 50.0, 700.0 - i as f32 * 13.0, 11.0, false, 1))
            .collect();
        // Mesma largura, fonte maior: continua sendo texto.
        els.push(em(&cheia, 50.0, 380.0, 13.0, true, 1));
        // Curta e na mesma fonte maior: aí sim é título.
        els.push(em("Consideracoes finais", 50.0, 360.0, 13.0, true, 1));

        let md = elements_to_markdown(&els);
        assert!(
            !md.contains(&format!("# {}", cheia.trim())),
            "linha de largura cheia não é título, saiu:\n{md}"
        );
        assert!(md.contains("# Consideracoes finais"), "título curto deveria continuar título:\n{md}");
    }

    /// Documento curto não tem amostra para medir largura — a régua não se
    /// aplica, e o título continua valendo pelo tamanho da fonte.
    #[test]
    fn documento_curto_nao_sofre_a_regua_de_largura() {
        let els = vec![
            em("Um titulo qualquer", 50.0, 700.0, 18.0, true, 1),
            em("Corpo do texto.", 50.0, 680.0, 10.0, false, 1),
        ];
        assert!(elements_to_markdown(&els).starts_with("# Um titulo qualquer"));
    }

    /// Cabeçalho/rodapé se repete em toda página, na mesma altura. É elemento
    /// de página, não conteúdo — e entra como parágrafo solto no meio do texto.
    #[test]
    fn cabecalho_repetido_em_varias_paginas_e_descartado() {
        let mut els = Vec::new();
        for pagina in 1..=3 {
            els.push(em("BOLETIM INTERNO", 50.0, 780.0, 8.0, false, pagina));
            els.push(em(&format!("Conteudo proprio da pagina {pagina}."), 50.0, 700.0, 11.0, false, pagina));
        }
        let md = elements_to_markdown(&els);
        assert!(!md.contains("BOLETIM INTERNO"), "cabeçalho de página não é conteúdo, saiu:\n{md}");
        assert!(md.contains("Conteudo proprio da pagina 2."));
    }

    /// Texto que se repete mas **não** está na mesma altura é conteúdo — não
    /// pode ser confundido com elemento de página.
    #[test]
    fn repeticao_em_alturas_diferentes_e_conteudo() {
        let mut els = Vec::new();
        for (pagina, y) in [(1u32, 700.0f32), (2, 500.0), (3, 300.0)] {
            els.push(em("Total apurado no periodo", 50.0, y, 11.0, false, pagina));
        }
        let md = elements_to_markdown(&els);
        assert_eq!(md.matches("Total apurado no periodo").count(), 3);
    }

    // --- Fixtures -----------------------------------------------------------

    #[test]
    fn test_extract_simple_headings() {
        let elements = extract_pdf_text_with_positions(&fixture_path("simple_headings.pdf")).unwrap();
        assert!(!elements.is_empty(), "Deveria extrair elementos de texto");

        let all_text: String = elements.iter().map(|e| e.text.clone()).collect();
        assert!(all_text.contains("Relatório"), "Texto com acento deve ser decodificado corretamente");
        assert!(all_text.contains("Trimestre"));

        let sizes: Vec<f32> = elements.iter().map(|e| e.font_size).collect();
        let max_size = sizes.iter().cloned().fold(0.0f32, f32::max);
        let min_size = sizes.iter().cloned().fold(f32::INFINITY, f32::min);
        assert!(max_size > min_size, "Deveria ter tamanhos de fonte diferentes");
    }

    #[test]
    fn test_extract_all_fixtures() {
        let fixtures = [
            "simple_headings.pdf",
            "table_simple.pdf",
            "table_merged.pdf",
            "two_column.pdf",
            "code_block.pdf",
            "image_caption.pdf",
            "scanned.pdf",
        ];
        for fixture in fixtures {
            let elements = extract_pdf_text_with_positions(&fixture_path(fixture))
                .unwrap_or_else(|e| panic!("Failed to extract {fixture}: {e}"));
            if fixture != "scanned.pdf" {
                assert!(!elements.is_empty(), "{fixture} deveria ter texto");
            }
        }
    }

    #[test]
    fn test_import_pdf_to_markdown_command() {
        let result = import_pdf_to_markdown(&fixture_path("simple_headings.pdf"));
        assert!(result.is_ok(), "import_pdf_to_markdown deveria retornar Ok");
        let markdown = result.unwrap();
        assert!(!markdown.is_empty(), "Markdown não deveria estar vazio");
        assert!(markdown.contains("Relatório"), "Deveria conter 'Relatório'");
    }

    #[test]
    fn test_import_pdf_to_markdown_has_headings() {
        let markdown = import_pdf_to_markdown(&fixture_path("simple_headings.pdf")).unwrap();
        assert!(markdown.contains("# Relatório de Vendas"), "faltou H1, saiu:\n{markdown}");
        assert!(markdown.contains("## Primeiro Trimestre"), "faltou H2, saiu:\n{markdown}");
    }
}


