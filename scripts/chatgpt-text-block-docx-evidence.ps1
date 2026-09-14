#requires -Version 7.0

function Get-ChatGptTextBlockDocxFixture {
    $fixture=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'fixtures/chatgpt-text-block-docx.json') -Raw | ConvertFrom-Json
    if($fixture.schema -cne 'elon.text_block_docx_fixture.v1' -or
        -not $fixture.source.StartsWith("# ELON_TEXT_BLOCK_ACCEPTANCE_V1`n", [StringComparison]::Ordinal)){
        throw 'docx_fixture_invalid'
    }
    $bytes=[Text.UTF8Encoding]::new($false,$true).GetBytes($fixture.source)
    [pscustomobject]@{source_base64=[Convert]::ToBase64String($bytes);paragraphs=$fixture.paragraphs;
        source_sha256=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()}
}

function Assert-ChatGptDocxExportReceipt {
    param($Receipt,$Fixture)
    if($Receipt.exported -isnot [bool] -or -not $Receipt.exported -or
        $Receipt.extension -cne 'docx' -or $Receipt.source_sha256 -cne $Fixture.source_sha256){
        throw 'docx_export_receipt_mismatch'
    }
}

function Read-ChatGptDocxXml {
    param([IO.Compression.ZipArchiveEntry]$Entry)
    if($Entry.Length -gt 524288){throw 'docx_part_too_large'}
    $stream=$Entry.Open();$memory=[IO.MemoryStream]::new()
    try {
        $buffer=[byte[]]::new(8192)
        while(($count=$stream.Read($buffer,0,$buffer.Length)) -gt 0){
            if($memory.Length+$count -gt 524288){throw 'docx_part_too_large'}
            $memory.Write($buffer,0,$count)
        }
        if($memory.Length -ne $Entry.Length){throw 'docx_part_length_mismatch'}
        $memory.Position=0
        $settings=[Xml.XmlReaderSettings]::new()
        $settings.DtdProcessing=[Xml.DtdProcessing]::Prohibit
        $settings.XmlResolver=$null;$settings.MaxCharactersInDocument=524288
        $reader=[Xml.XmlReader]::Create($memory,$settings)
        try {$doc=[Xml.XmlDocument]::new();$doc.XmlResolver=$null;$doc.Load($reader);return ,$doc}
        catch {throw 'docx_xml_invalid'}
        finally {$reader.Dispose()}
    } finally {$memory.Dispose();$stream.Dispose()}
}

function Assert-ChatGptDocxExportFile {
    param([Parameter(Mandatory)][string]$Path,[Parameter(Mandatory)][string]$ExpectedFileHash,$Fixture)
    if($ExpectedFileHash -cnotmatch '^[a-f0-9]{64}$' -or (Get-Item -LiteralPath $Path).Length -gt 1048576){
        throw 'docx_file_boundary'
    }
    if((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $ExpectedFileHash){
        throw 'docx_file_hash_mismatch'
    }
    $names=@('[Content_Types].xml','_rels/.rels','word/document.xml','word/styles.xml',
        'word/numbering.xml','word/_rels/document.xml.rels')
    $zip=[IO.Compression.ZipFile]::OpenRead($Path)
    try {
        if($zip.Entries.Count -ne $names.Count){throw 'docx_parts_mismatch'}
        $parts=[Collections.Generic.Dictionary[string,Xml.XmlDocument]]::new([StringComparer]::Ordinal)
        foreach($entry in $zip.Entries){
            if($entry.FullName -cnotin $names -or $parts.ContainsKey($entry.FullName)){throw 'docx_parts_mismatch'}
            $parts.Add($entry.FullName,(Read-ChatGptDocxXml $entry))
        }
        $word='http://schemas.openxmlformats.org/wordprocessingml/2006/main'
        $office='http://schemas.openxmlformats.org/officeDocument/2006/relationships/'
        $ns=[Xml.XmlNamespaceManager]::new($parts['word/document.xml'].NameTable)
        $ns.AddNamespace('w',$word)
        $ns.AddNamespace('ct','http://schemas.openxmlformats.org/package/2006/content-types')
        $ns.AddNamespace('r','http://schemas.openxmlformats.org/package/2006/relationships')
        $types=$parts['[Content_Types].xml'].SelectNodes('/ct:Types/ct:Override',$ns)
        $defaults=$parts['[Content_Types].xml'].SelectNodes('/ct:Types/ct:Default',$ns)
        $defaultTypes=@{xml='application/xml';rels='application/vnd.openxmlformats-package.relationships+xml'}
        if($defaults.Count -ne 2){throw 'docx_content_types_mismatch'}
        foreach($key in $defaultTypes.Keys){
            $type=@($defaults | Where-Object {$_.GetAttribute('Extension') -ceq $key})
            if($type.Count -ne 1 -or $type[0].GetAttribute('ContentType') -cne $defaultTypes[$key]){
                throw 'docx_content_types_mismatch'
            }
        }
        $expectedTypes=@{document='document.main';styles='styles';numbering='numbering'}
        if($types.Count -ne 3){throw 'docx_content_types_mismatch'}
        foreach($key in $expectedTypes.Keys){
            $type=@($types | Where-Object {$_.GetAttribute('PartName') -ceq "/word/$key.xml"})
            if($type.Count -ne 1 -or $type[0].GetAttribute('ContentType') -cne
                "application/vnd.openxmlformats-officedocument.wordprocessingml.$($expectedTypes[$key])+xml"){
                throw 'docx_content_types_mismatch'
            }
        }
        foreach($name in @('_rels/.rels','word/_rels/document.xml.rels')){
            $expected=if($name -ceq '_rels/.rels'){@{officeDocument='word/document.xml'}}else{@{styles='styles.xml';numbering='numbering.xml'}}
            $links=$parts[$name].SelectNodes('/r:Relationships/r:Relationship',$ns)
            if($links.Count -ne $expected.Count){throw 'docx_relationship_mismatch'}
            foreach($key in $expected.Keys){
                $link=@($links | Where-Object {$_.GetAttribute('Type') -ceq ($office+$key)})
                if($link.Count -ne 1 -or $link[0].HasAttribute('TargetMode') -or
                    $link[0].GetAttribute('Target') -cne $expected[$key]){throw 'docx_relationship_mismatch'}
            }
        }
        $doc=$parts['word/document.xml']
        $styles=$parts['word/styles.xml']
        if($styles.SelectNodes('/w:styles/w:style[@w:type="paragraph"][@w:styleId="Heading1"]',$ns).Count -ne 1 -or
            $styles.SelectNodes('/w:styles/w:style[@w:type="paragraph"][@w:styleId="Normal"]',$ns).Count -ne 1){
            throw 'docx_style_definition_mismatch'
        }
        $paragraphs=$doc.SelectNodes('/w:document/w:body//w:p',$ns)
        if($paragraphs.Count -ne $Fixture.paragraphs.Count){throw 'docx_paragraph_count_mismatch'}
        for($i=0;$i -lt $paragraphs.Count;$i++){
            $value=[Text.StringBuilder]::new()
            foreach($run in $paragraphs[$i].SelectNodes('.//w:t|.//w:br|.//w:tab',$ns)){
                switch($run.LocalName){t{[void]$value.Append($run.InnerText)}br{[void]$value.Append("`n")}tab{[void]$value.Append("`t")}}
            }
            if($value.ToString() -cne $Fixture.paragraphs[$i]){throw 'docx_paragraph_content_mismatch'}
        }
        if($doc.SelectNodes('//w:tbl',$ns).Count -ne 1 -or $doc.SelectNodes('//w:tr',$ns).Count -ne 2 -or
            $doc.SelectNodes('//w:tc',$ns).Count -ne 4 -or $doc.SelectNodes('//w:numPr',$ns).Count -ne 2 -or
            $doc.SelectNodes('//w:numPr[w:ilvl/@w:val="0"][w:numId/@w:val="1"]',$ns).Count -ne 2 -or
            $doc.SelectNodes('//w:pStyle[@w:val="Heading1"]',$ns).Count -ne 1 -or
            $doc.SelectNodes('//w:r[w:t="Bold"]/w:rPr/w:b',$ns).Count -ne 1 -or
            $doc.SelectNodes('//w:r[w:t="italic"]/w:rPr/w:i',$ns).Count -ne 1){throw 'docx_structure_mismatch'}
        $numbering=$parts['word/numbering.xml']
        if($numbering.SelectNodes('/w:numbering/w:abstractNum',$ns).Count -ne 1 -or
            $numbering.SelectNodes('/w:numbering/w:abstractNum[@w:abstractNumId="1"]/w:lvl[@w:ilvl="0"][w:start/@w:val="3"][w:numFmt/@w:val="decimal"][w:lvlText/@w:val="%1."]',$ns).Count -ne 1 -or
            $numbering.SelectNodes('/w:numbering/w:num',$ns).Count -ne 1 -or
            $numbering.SelectNodes('/w:numbering/w:num[@w:numId="1"]/w:abstractNumId[@w:val="1"]',$ns).Count -ne 1){throw 'docx_numbering_mismatch'}
        [pscustomobject]@{file_sha256=$ExpectedFileHash;package_valid=$true;content_matches=$true;
            paragraphs=$paragraphs.Count;tables=1;list_items=2;private_content_exported=$false}
    } finally {$zip.Dispose()}
}
