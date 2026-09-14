package com.elon.app

import java.io.OutputStream
import org.w3c.dom.Document
import org.w3c.dom.Element
import org.w3c.dom.Node
import org.xmlpull.v1.XmlPullParserFactory
import org.xmlpull.v1.XmlSerializer

/** Serializes our generated OOXML parts without Android's surrogate-splitting Transformer. */
internal object WebChatTextBlockXml {
    private const val XML = "http://www.w3.org/XML/1998/namespace"

    fun write(document: Document, output: OutputStream) {
        val root = requireNotNull(document.documentElement)
        val namespace = requireNotNull(root.namespaceURI)
        val serializer = XmlPullParserFactory.newInstance().newSerializer()
        serializer.setOutput(output, "UTF-8")
        serializer.startDocument("UTF-8", true)
        serializer.setPrefix(root.prefix.orEmpty(), namespace)
        element(serializer, root, namespace, 0)
        serializer.endDocument()
    }

    private fun element(serializer: XmlSerializer, node: Element, namespace: String, depth: Int) {
        require(depth <= 128 && node.namespaceURI == namespace) { "Unexpected document structure" }
        serializer.startTag(namespace, node.localName)
        for (index in 0 until node.attributes.length) {
            val attribute = node.attributes.item(index)
            val attributeNamespace = attribute.namespaceURI.orEmpty()
            require(attributeNamespace.isEmpty() || attributeNamespace == namespace || attributeNamespace == XML) {
                "Unexpected document attribute"
            }
            serializer.attribute(attributeNamespace, attribute.localName ?: attribute.nodeName, attribute.nodeValue)
        }
        for (index in 0 until node.childNodes.length) {
            val child = node.childNodes.item(index)
            when (child.nodeType) {
                Node.ELEMENT_NODE -> element(serializer, child as Element, namespace, depth + 1)
                Node.TEXT_NODE -> serializer.text(child.nodeValue)
                else -> throw IllegalArgumentException("Unexpected document node")
            }
        }
        serializer.endTag(namespace, node.localName)
    }
}
